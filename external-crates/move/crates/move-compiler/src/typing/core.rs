// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    debug_display, diag,
    diagnostics::{
        codes::{NameResolution, TypeSafety},
        Diagnostic,
    },
    expansion::ast::{AbilitySet, ModuleIdent, ModuleIdent_, Visibility},
    ice,
    naming::ast::{
        self as N, BlockLabel, BuiltinTypeName_, Color, RefVar, ResolvedUseFuns, StructDefinition,
        StructTypeParameter, TParam, TParamID, TVar, Type, TypeName, TypeName_, Type_, UseFunKind,
        Var,
    },
    parser::ast::{
        Ability_, ConstantName, Field, FunctionName, Mutability, StructName, ENTRY_MODIFIER,
    },
    shared::{known_attributes::TestingAttribute, program_info::*, unique_map::UniqueMap, *},
    FullyCompiledProgram,
};
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
};

//**************************************************************************************************
// Context
//**************************************************************************************************

pub struct UseFunsScope {
    color: Option<Color>,
    count: usize,
    use_funs: ResolvedUseFuns,
}

pub enum Constraint {
    AbilityConstraint {
        loc: Loc,
        msg: Option<String>,
        ty: Type,
        constraints: AbilitySet,
    },
    NumericConstraint(Loc, &'static str, Type),
    BitsConstraint(Loc, &'static str, Type),
    OrderedConstraint(Loc, &'static str, Type),
    BaseTypeConstraint(Loc, String, Type),
    SingleTypeConstraint(Loc, String, Type),
}
pub type Constraints = Vec<Constraint>;
pub type TParamSubst = HashMap<TParamID, Type>;

pub struct Local {
    pub mut_: Mutability,
    pub ty: Type,
    pub used_mut: Option<Loc>,
}

#[derive(Debug)]
pub struct MacroCall {
    pub module: ModuleIdent,
    pub function: FunctionName,
    pub invocation: Loc,
    pub scope_color: Color,
}

#[derive(Debug)]
pub enum MacroExpansion {
    Call(Box<MacroCall>),
    // An argument to a macro, where the entire expression was substituted in
    Argument { scope_color: Color },
}

pub struct Context<'env> {
    pub modules: NamingProgramInfo,
    macros: UniqueMap<ModuleIdent, UniqueMap<FunctionName, N::Sequence>>,
    pub env: &'env mut CompilationEnv,

    use_funs: Vec<UseFunsScope>,
    pub current_package: Option<Symbol>,
    pub current_module: Option<ModuleIdent>,
    pub current_function: Option<FunctionName>,
    pub in_macro_function: bool,
    max_variable_color: RefCell<u16>,
    pub return_type: Option<Type>,
    locals: UniqueMap<Var, Local>,

    pub subst: Subst,
    pub constraints: Constraints,

    named_block_map: BTreeMap<BlockLabel, Type>,

    /// collects all friends that should be added over the course of 'public(package)' calls
    /// structured as (defining module, new friend, location) where `new friend` is usually the
    /// context's current module. Note there may be more than one location in practice, but
    /// tracking a single one is sufficient for error reporting.
    pub new_friends: BTreeSet<(ModuleIdent, Loc)>,
    /// collects all used module members (functions and constants) but it's a superset of these in
    /// that it may contain other identifiers that do not in fact represent a function or a constant
    pub used_module_members: BTreeMap<ModuleIdent_, BTreeSet<Symbol>>,
    /// Current macros being expanded
    pub macro_expansion: Vec<MacroExpansion>,
    /// Stack of items from `macro_expansion` pushed/popped when entering/leaving a lambda expansion
    /// This is to prevent accidentally thinking we are in a recursive call if a macro is used
    /// inside a lambda body
    pub lambda_expansion: Vec<Vec<MacroExpansion>>,
}

pub struct ResolvedFunctionType {
    pub declared: Loc,
    pub macro_: Option<Loc>,
    pub ty_args: Vec<Type>,
    pub params: Vec<(Var, Type)>,
    pub return_: Type,
}

impl UseFunsScope {
    pub fn global(info: &NamingProgramInfo) -> Self {
        let count = 1;
        let mut use_funs = BTreeMap::new();
        for (_, _, minfo) in &info.modules {
            for (tn, methods) in &minfo.use_funs {
                let public_methods = methods.ref_filter_map(|_, uf| {
                    if uf.is_public.is_some() {
                        Some(uf.clone())
                    } else {
                        None
                    }
                });
                if public_methods.is_empty() {
                    continue;
                }

                assert!(
                    !use_funs.contains_key(tn),
                    "ICE public methods should have been filtered to the defining module.
                    tn: {tn}.
                    prev: {}
                    new: {}",
                    debug_display!((tn, (use_funs.get(tn).unwrap()))),
                    debug_display!((tn, &public_methods))
                );
                use_funs.insert(tn.clone(), public_methods);
            }
        }
        UseFunsScope {
            color: None,
            count,
            use_funs,
        }
    }
}

impl<'env> Context<'env> {
    pub fn new(
        env: &'env mut CompilationEnv,
        _pre_compiled_lib: Option<&FullyCompiledProgram>,
        info: NamingProgramInfo,
    ) -> Self {
        let global_use_funs = UseFunsScope::global(&info);
        Context {
            use_funs: vec![global_use_funs],
            subst: Subst::empty(),
            current_package: None,
            current_module: None,
            current_function: None,
            in_macro_function: false,
            max_variable_color: RefCell::new(0),
            return_type: None,
            constraints: vec![],
            locals: UniqueMap::new(),
            modules: info,
            macros: UniqueMap::new(),
            named_block_map: BTreeMap::new(),
            env,
            new_friends: BTreeSet::new(),
            used_module_members: BTreeMap::new(),
            macro_expansion: vec![],
            lambda_expansion: vec![],
        }
    }

    pub fn set_macros(
        &mut self,
        macros: UniqueMap<ModuleIdent, UniqueMap<FunctionName, N::Sequence>>,
    ) {
        debug_assert!(self.macros.is_empty());
        self.macros = macros;
    }

    pub fn add_use_funs_scope(&mut self, new_scope: N::UseFuns) {
        let N::UseFuns {
            color,
            resolved: new_scope,
            implicit_candidates,
        } = new_scope;
        assert!(
            implicit_candidates.is_empty(),
            "ICE use fun candidates should have been resolved"
        );
        let cur = self.use_funs.last_mut().unwrap();
        if new_scope.is_empty() && cur.color == Some(color) {
            cur.count += 1;
            return;
        }
        self.use_funs.push(UseFunsScope {
            count: 1,
            use_funs: new_scope,
            color: Some(color),
        })
    }

    pub fn pop_use_funs_scope(&mut self) -> N::UseFuns {
        let cur = self.use_funs.last_mut().unwrap();
        if cur.count > 1 {
            cur.count -= 1;
            return N::UseFuns::new(cur.color.unwrap_or(0));
        }
        let UseFunsScope {
            use_funs, color, ..
        } = self.use_funs.pop().unwrap();
        for (tn, methods) in use_funs.iter() {
            let unused = methods.iter().filter(|(_, _, uf)| !uf.used);
            for (_, method, use_fun) in unused {
                let N::UseFun {
                    loc,
                    kind,
                    attributes: _,
                    is_public: _,
                    tname: _,
                    target_function: _,
                    used: _,
                } = use_fun;
                let msg = match kind {
                    UseFunKind::Explicit => {
                        format!("Unused 'use fun' of '{tn}.{method}'. Consider removing it")
                    }
                    UseFunKind::UseAlias => {
                        format!("Unused 'use' of alias '{method}'. Consider removing it")
                    }
                    UseFunKind::FunctionDeclaration => {
                        panic!("ICE function declaration use funs should never be added to use fun")
                    }
                };
                self.env.add_diag(diag!(UnusedItem::Alias, (*loc, msg)))
            }
        }
        N::UseFuns {
            resolved: use_funs,
            color: color.unwrap_or(0),
            implicit_candidates: UniqueMap::new(),
        }
    }

    pub fn find_method_and_mark_used(
        &mut self,
        tn: &TypeName,
        method: Name,
    ) -> Option<(ModuleIdent, FunctionName)> {
        let cur_color = self.use_funs.last().unwrap().color;
        self.use_funs.iter_mut().rev().find_map(|scope| {
            // scope color is None for global scope, which is always in consideration
            // otherwise, the color must match the current color. In practice, we are preventing
            // macro scopes from interfering with each the scopes in which they are expanded
            if scope.color.is_some() && scope.color != cur_color {
                return None;
            }
            let use_fun = scope.use_funs.get_mut(tn)?.get_mut(&method)?;
            use_fun.used = true;
            Some(use_fun.target_function)
        })
    }

    /// true iff it is safe to expand,
    /// false with an error otherwise (e.g. a recursive expansion)
    pub fn add_macro_expansion(&mut self, m: ModuleIdent, f: FunctionName, loc: Loc) -> bool {
        let current_call_color = self.current_call_color();

        let mut prev_opt = None;
        for (idx, mexp) in self.macro_expansion.iter().enumerate().rev() {
            match mexp {
                MacroExpansion::Argument { scope_color } => {
                    // the argument has a smaller (or equal) color, meaning this lambda/arg was
                    // written in an outer scope
                    if current_call_color > *scope_color {
                        break;
                    }
                }
                MacroExpansion::Call(c) => {
                    let MacroCall {
                        module,
                        function,
                        scope_color,
                        ..
                    } = &**c;
                    // If we find a call (same module/fn) above us at a shallower expansion depth,
                    // without an interceding macro arg/lambda, we are in a macro calling itself.
                    // If it was a deeper depth, that's fine -- it must have come from elsewhere.
                    if current_call_color > *scope_color && module == &m && function == &f {
                        prev_opt = Some(idx);
                        break;
                    }
                }
            }
        }

        if let Some(idx) = prev_opt {
            let msg = format!(
                "Recursive macro expansion. '{}::{}' cannot recursively expand itself",
                m, f
            );
            let mut diag = diag!(TypeSafety::CannotExpandMacro, (loc, msg));
            let cycle = self.macro_expansion[idx..]
                .iter()
                .filter_map(|case| match case {
                    MacroExpansion::Call(c) => Some((&c.module, &c.function, &c.invocation)),
                    MacroExpansion::Argument { .. } => None,
                });
            for (prev_m, prev_f, prev_loc) in cycle {
                let msg = if prev_m == &m && prev_f == &f {
                    format!("'{}::{}' previously expanded here", prev_m, prev_f)
                } else {
                    "From this macro expansion".to_owned()
                };
                diag.add_secondary_label((*prev_loc, msg));
            }
            self.env.add_diag(diag);
            false
        } else {
            self.macro_expansion
                .push(MacroExpansion::Call(Box::new(MacroCall {
                    module: m,
                    function: f,
                    invocation: loc,
                    scope_color: current_call_color,
                })));
            true
        }
    }

    pub fn pop_macro_expansion(&mut self, m: &ModuleIdent, f: &FunctionName) {
        let Some(MacroExpansion::Call(c)) = self.macro_expansion.pop() else {
            panic!("ICE macro expansion stack should have a call when leaving a macro expansion")
        };
        let MacroCall {
            module, function, ..
        } = *c;
        assert!(
            m == &module && f == &function,
            "ICE macro expansion stack should be popped in reverse order"
        );
    }

    pub fn maybe_enter_macro_argument(
        &mut self,
        from_macro_argument: Option<N::MacroArgument>,
        color: Color,
    ) {
        if from_macro_argument.is_some() {
            self.macro_expansion
                .push(MacroExpansion::Argument { scope_color: color })
        }
    }

    pub fn maybe_exit_macro_argument(&mut self, from_macro_argument: Option<N::MacroArgument>) {
        if from_macro_argument.is_some() {
            let Some(MacroExpansion::Argument { .. }) = self.macro_expansion.pop() else {
                panic!("ICE macro expansion stack should have a lambda when leaving a lambda")
            };
        }
    }

    pub fn current_call_color(&self) -> Color {
        self.use_funs.last().unwrap().color.unwrap()
    }

    pub fn reset_for_module_item(&mut self) {
        self.named_block_map = BTreeMap::new();
        self.return_type = None;
        self.locals = UniqueMap::new();
        self.subst = Subst::empty();
        self.constraints = Constraints::new();
        self.current_function = None;
        self.in_macro_function = false;
        self.max_variable_color = RefCell::new(0);
        self.macro_expansion = vec![];
        self.lambda_expansion = vec![];
    }

    pub fn error_type(&mut self, loc: Loc) -> Type {
        sp(loc, Type_::UnresolvedError)
    }

    pub fn add_ability_constraint(
        &mut self,
        loc: Loc,
        msg_opt: Option<impl Into<String>>,
        ty: Type,
        ability_: Ability_,
    ) {
        self.add_ability_set_constraint(
            loc,
            msg_opt,
            ty,
            AbilitySet::from_abilities(vec![sp(loc, ability_)]).unwrap(),
        )
    }

    pub fn add_ability_set_constraint(
        &mut self,
        loc: Loc,
        msg_opt: Option<impl Into<String>>,
        ty: Type,
        constraints: AbilitySet,
    ) {
        self.constraints.push(Constraint::AbilityConstraint {
            loc,
            msg: msg_opt.map(|s| s.into()),
            ty,
            constraints,
        })
    }

    pub fn add_base_type_constraint(&mut self, loc: Loc, msg: impl Into<String>, t: Type) {
        self.constraints
            .push(Constraint::BaseTypeConstraint(loc, msg.into(), t))
    }

    pub fn add_single_type_constraint(&mut self, loc: Loc, msg: impl Into<String>, t: Type) {
        self.constraints
            .push(Constraint::SingleTypeConstraint(loc, msg.into(), t))
    }

    pub fn add_numeric_constraint(&mut self, loc: Loc, op: &'static str, t: Type) {
        self.constraints
            .push(Constraint::NumericConstraint(loc, op, t))
    }

    pub fn add_bits_constraint(&mut self, loc: Loc, op: &'static str, t: Type) {
        self.constraints
            .push(Constraint::BitsConstraint(loc, op, t))
    }

    pub fn add_ordered_constraint(&mut self, loc: Loc, op: &'static str, t: Type) {
        self.constraints
            .push(Constraint::OrderedConstraint(loc, op, t))
    }

    pub fn declare_local(&mut self, mut_: Mutability, var: Var, ty: Type) {
        let local = Local {
            mut_,
            ty,
            used_mut: None,
        };
        if let Err((_, prev_loc)) = self.locals.add(var, local) {
            let msg = format!("ICE duplicate {var:?}. Should have been made unique in naming");
            self.env
                .add_diag(ice!((var.loc, msg), (prev_loc, "Previously declared here")));
        }
    }

    pub fn get_local_type(&mut self, var: &Var) -> Type {
        if !self.locals.contains_key(var) {
            let msg = format!("ICE unbound {var:?}. Should have failed in naming");
            self.env.add_diag(ice!((var.loc, msg)));
            return self.error_type(var.loc);
        }

        self.locals.get(var).unwrap().ty.clone()
    }

    pub fn mark_mutable_usage(&mut self, loc: Loc, var: &Var) -> (Loc, Mutability) {
        if !self.locals.contains_key(var) {
            let msg = format!("ICE unbound {var:?}. Should have failed in naming");
            self.env.add_diag(ice!((loc, msg)));
            return (loc, Mutability::None);
        }

        // should not fail, already checked in naming
        let decl_loc = *self.locals.get_loc(var).unwrap();
        let local = self.locals.get_mut(var).unwrap();
        local.used_mut = Some(loc);
        (decl_loc, local.mut_)
    }

    pub fn take_locals(&mut self) -> UniqueMap<Var, Local> {
        std::mem::take(&mut self.locals)
    }

    pub fn is_current_module(&self, m: &ModuleIdent) -> bool {
        match &self.current_module {
            Some(curm) => curm == m,
            None => false,
        }
    }

    pub fn is_current_function(&self, m: &ModuleIdent, f: &FunctionName) -> bool {
        self.is_current_module(m) && matches!(&self.current_function, Some(curf) if curf == f)
    }

    pub fn current_package(&self) -> Option<Symbol> {
        self.current_module
            .as_ref()
            .and_then(|mident| self.module_info(mident).package)
    }

    // `loc` indicates the location that caused the add to occur
    fn record_current_module_as_friend(&mut self, m: &ModuleIdent, loc: Loc) {
        if matches!(self.current_module, Some(current_mident) if m != &current_mident) {
            self.new_friends.insert((*m, loc));
        }
    }

    fn current_module_shares_package_and_address(&self, m: &ModuleIdent) -> bool {
        self.current_module.is_some_and(|current_mident| {
            m.value.address == current_mident.value.address
                && self.module_info(m).package == self.module_info(&current_mident).package
        })
    }

    fn current_module_is_a_friend_of(&self, m: &ModuleIdent) -> bool {
        match &self.current_module {
            None => false,
            Some(current_mident) => {
                let minfo = self.module_info(m);
                minfo.friends.contains_key(current_mident)
            }
        }
    }

    /// current_module.is_test_only || current_function.is_test_only || current_function.is_test
    fn is_testing_context(&self) -> bool {
        self.current_module.as_ref().is_some_and(|m| {
            let minfo = self.module_info(m);
            let is_test_only = minfo.attributes.is_test_or_test_only();
            is_test_only
                || self.current_function.as_ref().is_some_and(|f| {
                    let finfo = minfo.functions.get(f).unwrap();
                    finfo.attributes.is_test_or_test_only()
                })
        })
    }

    fn module_info(&self, m: &ModuleIdent) -> &ModuleInfo {
        self.modules.module(m)
    }

    fn struct_definition(&self, m: &ModuleIdent, n: &StructName) -> &StructDefinition {
        self.modules.struct_definition(m, n)
    }

    pub fn struct_declared_abilities(&self, m: &ModuleIdent, n: &StructName) -> &AbilitySet {
        self.modules.struct_declared_abilities(m, n)
    }

    pub fn struct_declared_loc(&self, m: &ModuleIdent, n: &StructName) -> Loc {
        self.modules.struct_declared_loc(m, n)
    }

    pub fn struct_tparams(&self, m: &ModuleIdent, n: &StructName) -> &Vec<StructTypeParameter> {
        self.modules.struct_type_parameters(m, n)
    }

    pub fn function_info(&self, m: &ModuleIdent, n: &FunctionName) -> &FunctionInfo {
        self.modules.function_info(m, n)
    }

    pub fn macro_body(&self, m: &ModuleIdent, n: &FunctionName) -> Option<&N::Sequence> {
        self.macros.get(m)?.get(n)
    }

    fn constant_info(&mut self, m: &ModuleIdent, n: &ConstantName) -> &ConstantInfo {
        let constants = &self.module_info(m).constants;
        constants.get(n).expect("ICE should have failed in naming")
    }

    // pass in a location for a better error location
    pub fn named_block_type(&mut self, name: BlockLabel, loc: Loc) -> Type {
        if let Some(ty) = self.named_block_map.get(&name) {
            ty.clone()
        } else {
            let new_type = make_tvar(self, loc);
            self.named_block_map.insert(name, new_type.clone());
            new_type
        }
    }

    pub fn named_block_type_opt(&self, name: BlockLabel) -> Option<Type> {
        self.named_block_map.get(&name).cloned()
    }

    pub fn next_variable_color(&mut self) -> Color {
        let max_variable_color: &mut u16 = &mut self.max_variable_color.borrow_mut();
        *max_variable_color += 1;
        *max_variable_color
    }

    pub fn set_max_variable_color(&self, color: Color) {
        let max_variable_color: &mut u16 = &mut self.max_variable_color.borrow_mut();
        assert!(
            *max_variable_color <= color,
            "ICE a new, lower color means reusing variables \
            {} <= {}",
            *max_variable_color,
            color,
        );
        *max_variable_color = color;
    }
}

//**************************************************************************************************
// Subst
//**************************************************************************************************

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefKind {
    Value,
    ImmRef,
    MutRef,
    Forward(RefVar),
}

#[derive(Clone, Debug)]
pub struct Subst {
    tvars: HashMap<TVar, Type>,
    num_vars: HashMap<TVar, Loc>,
    ref_vars: HashMap<RefVar, RefKind>,
}

impl Subst {
    pub fn empty() -> Self {
        Self {
            tvars: HashMap::new(),
            num_vars: HashMap::new(),
            ref_vars: HashMap::new(),
        }
    }

    pub fn insert(&mut self, tvar: TVar, bt: Type) {
        self.tvars.insert(tvar, bt);
    }

    pub fn get(&self, tvar: TVar) -> Option<&Type> {
        self.tvars.get(&tvar)
    }

    pub fn new_num_var(&mut self, loc: Loc) -> TVar {
        let tvar = TVar::next();
        assert!(self.num_vars.insert(tvar, loc).is_none());
        tvar
    }

    pub fn set_num_var(&mut self, tvar: TVar, loc: Loc) {
        self.num_vars.entry(tvar).or_insert(loc);
        if let Some(sp!(_, Type_::Var(next))) = self.get(tvar) {
            let next = *next;
            self.set_num_var(next, loc)
        }
    }

    pub fn is_num_var(&self, tvar: TVar) -> bool {
        self.num_vars.contains_key(&tvar)
    }

    pub fn insert_ref_var(&mut self, rvar: RefVar, rk: RefKind) {
        self.ref_vars.insert(rvar, rk);
    }

    pub fn get_ref_var(&self, rvar: RefVar) -> Option<&RefKind> {
        self.ref_vars.get(&rvar)
    }
}

impl ast_debug::AstDebug for Subst {
    fn ast_debug(&self, w: &mut ast_debug::AstWriter) {
        let Subst {
            tvars,
            num_vars,
            ref_vars,
        } = self;

        w.write("tvars:");
        w.indent(4, |w| {
            let mut tvars = tvars.iter().collect::<Vec<_>>();
            tvars.sort_by_key(|(v, _)| *v);
            for (tvar, bt) in tvars {
                w.write(&format!("{:?} => ", tvar));
                bt.ast_debug(w);
                w.new_line();
            }
        });
        w.write("num_vars:");
        w.indent(4, |w| {
            let mut num_vars = num_vars.keys().collect::<Vec<_>>();
            num_vars.sort();
            for tvar in num_vars {
                w.writeln(&format!("{:?}", tvar))
            }
        });
        w.write("ref_vars:");
        w.indent(4, |w| {
            let mut rvars = ref_vars.iter().collect::<Vec<_>>();
            rvars.sort_by_key(|(v, _)| *v);
            for (rvar, ref_kind) in rvars {
                w.write(&format!("{:?} => {:?}", rvar, ref_kind));
                w.new_line();
            }
        });
    }
}

impl RefKind {
    fn join(&self, other: &RefKind) -> Option<RefKind> {
        // val <: mut <: imm
        match (self, other) {
            (RefKind::Forward(_), _) => {
                panic!("ICE ref var should have been forwarded before join")
            }
            (_, RefKind::Forward(_)) => {
                panic!("ICE ref var should have been forwarded before join")
            }
            (RefKind::Value, RefKind::Value) => Some(RefKind::Value),
            (RefKind::MutRef, RefKind::MutRef) => Some(RefKind::MutRef),
            (RefKind::MutRef, RefKind::ImmRef) => Some(RefKind::ImmRef),
            (RefKind::ImmRef, RefKind::ImmRef) => Some(RefKind::ImmRef),
            (RefKind::ImmRef, RefKind::MutRef) => Some(RefKind::ImmRef),
            (RefKind::ImmRef, RefKind::Value) => None,
            (RefKind::MutRef, RefKind::Value) => None,
            (RefKind::Value, RefKind::ImmRef) => None,
            (RefKind::Value, RefKind::MutRef) => None,
        }
    }
}

//**************************************************************************************************
// Type error display
//**************************************************************************************************

pub fn error_format(b: &Type, subst: &Subst) -> String {
    error_format_impl(b, subst, false)
}

pub fn error_format_(b_: &Type_, subst: &Subst) -> String {
    error_format_impl_(b_, subst, false)
}

pub fn error_format_nested(b: &Type, subst: &Subst) -> String {
    error_format_impl(b, subst, true)
}

fn error_format_impl(sp!(_, b_): &Type, subst: &Subst, nested: bool) -> String {
    error_format_impl_(b_, subst, nested)
}

fn error_format_impl_(b_: &Type_, subst: &Subst, nested: bool) -> String {
    use Type_::*;
    let res = match b_ {
        UnresolvedError | Anything => "_".to_string(),
        Unit => "()".to_string(),
        Var(id) => {
            let last_id = forward_tvar(subst, *id);
            match subst.get(last_id) {
                Some(sp!(_, Var(_))) => unreachable!(),
                Some(t) => error_format_nested(t, subst),
                None if nested && subst.is_num_var(last_id) => "{integer}".to_string(),
                None if subst.is_num_var(last_id) => return "integer".to_string(),
                None => "_".to_string(),
            }
        }
        Apply(_, sp!(_, TypeName_::Multiple(_)), tys) => {
            let inner = format_comma(tys.iter().map(|s| error_format_nested(s, subst)));
            format!("({})", inner)
        }
        Apply(_, n, tys) => {
            let tys_str = if !tys.is_empty() {
                format!(
                    "<{}>",
                    format_comma(tys.iter().map(|t| error_format_nested(t, subst)))
                )
            } else {
                "".to_string()
            };
            format!("{}{}", n, tys_str)
        }
        Fun(args, result) => {
            format!(
                "|{}| -> {}",
                format_comma(args.iter().map(|t| error_format_nested(t, subst))),
                error_format_nested(result, subst)
            )
        }
        Param(tp) => tp.user_specified_name.value.to_string(),
        Ref(mut_, ty) => format!(
            "&{}{}",
            if *mut_ { "mut " } else { "" },
            error_format_nested(ty, subst)
        ),
        AutoRef(var, ty) => {
            let var = forward_ref_var(subst, *var);
            let ref_kind = match subst.get_ref_var(var) {
                Some(RefKind::Value) => "",
                Some(RefKind::ImmRef) => "&",
                Some(RefKind::MutRef) => "&mut ",
                Some(RefKind::Forward(_)) => unreachable!(),
                None => "auto ",
            };
            format!("{}{}", ref_kind, error_format_nested(ty, subst))
        }
    };
    if nested {
        res
    } else {
        format!("'{}'", res)
    }
}

//**************************************************************************************************
// Type utils
//**************************************************************************************************

pub fn infer_abilities<const INFO_PASS: bool>(
    context: &ProgramInfo<INFO_PASS>,
    subst: &Subst,
    ty: Type,
) -> AbilitySet {
    use Type_ as T;
    let loc = ty.loc;
    match unfold_type(subst, ty).value {
        T::Unit => AbilitySet::collection(loc),
        T::Ref(_, _) => AbilitySet::references(loc),
        T::Var(_) => unreachable!("ICE unfold_type failed, which is impossible"),
        T::AutoRef(_, _) => unreachable!("ICE autorefs should not survive local reasoning"),
        T::UnresolvedError | T::Anything => AbilitySet::all(loc),
        T::Param(TParam { abilities, .. }) | T::Apply(Some(abilities), _, _) => abilities,
        T::Apply(None, n, ty_args) => {
            let (declared_abilities, ty_args) = match &n.value {
                TypeName_::Multiple(_) => (AbilitySet::collection(loc), ty_args),
                TypeName_::Builtin(b) => (b.value.declared_abilities(b.loc), ty_args),
                TypeName_::ModuleType(m, n) => {
                    let declared_abilities = context.struct_declared_abilities(m, n).clone();
                    let non_phantom_ty_args = ty_args
                        .into_iter()
                        .zip(context.struct_type_parameters(m, n))
                        .filter(|(_, param)| !param.is_phantom)
                        .map(|(arg, _)| arg)
                        .collect::<Vec<_>>();
                    (declared_abilities, non_phantom_ty_args)
                }
            };
            let ty_args_abilities = ty_args
                .into_iter()
                .map(|ty| infer_abilities(context, subst, ty))
                .collect::<Vec<_>>();
            AbilitySet::from_abilities(declared_abilities.into_iter().filter(|ab| {
                let requirement = ab.value.requires();
                ty_args_abilities
                    .iter()
                    .all(|ty_arg_abilities| ty_arg_abilities.has_ability_(requirement))
            }))
            .unwrap()
        }
        T::Fun(_, _) => AbilitySet::functions(loc),
    }
}

// Returns
// - the declared location where abilities are added (if applicable)
// - the set of declared abilities
// - its type arguments
fn debug_abilities_info(context: &Context, ty: &Type) -> (Option<Loc>, AbilitySet, Vec<Type>) {
    use Type_ as T;
    let loc = ty.loc;
    match &ty.value {
        T::Unit | T::Ref(_, _) => (None, AbilitySet::references(loc), vec![]),
        T::Var(_) => panic!("ICE call unfold_type before debug_abilities_info"),
        T::AutoRef(_, _) => panic!("ICE autorefs should not survive local reasoning"),
        T::UnresolvedError | T::Anything => (None, AbilitySet::all(loc), vec![]),
        T::Param(TParam {
            abilities,
            user_specified_name,
            ..
        }) => (Some(user_specified_name.loc), abilities.clone(), vec![]),
        T::Apply(_, sp!(_, TypeName_::Multiple(_)), ty_args) => {
            (None, AbilitySet::collection(loc), ty_args.clone())
        }
        T::Apply(_, sp!(_, TypeName_::Builtin(b)), ty_args) => {
            (None, b.value.declared_abilities(b.loc), ty_args.clone())
        }
        T::Apply(_, sp!(_, TypeName_::ModuleType(m, n)), ty_args) => (
            Some(context.struct_declared_loc(m, n)),
            context.struct_declared_abilities(m, n).clone(),
            ty_args.clone(),
        ),
        T::Fun(_, _) => (None, AbilitySet::functions(loc), vec![]),
    }
}

pub fn make_num_tvar(context: &mut Context, loc: Loc) -> Type {
    let tvar = context.subst.new_num_var(loc);
    sp(loc, Type_::Var(tvar))
}

pub fn make_tvar(_context: &mut Context, loc: Loc) -> Type {
    sp(loc, Type_::Var(TVar::next()))
}

pub fn make_autoref(context: &mut Context, loc: Loc, ty: Type) -> (RefVar, Type) {
    use Type_::*;
    let rv = RefVar::next();
    match &ty.value {
        Ref(true, inner) => {
            context.subst.insert_ref_var(rv, RefKind::MutRef);
            (rv, sp(loc, AutoRef(rv, inner.clone())))
        }
        Ref(false, inner) => {
            context.subst.insert_ref_var(rv, RefKind::ImmRef);
            (rv, sp(loc, AutoRef(rv, inner.clone())))
        }
        Var(var) => {
            let var = forward_tvar(&context.subst, *var);
            if let Some(ty) = context.subst.get(var) {
                make_autoref(context, loc, ty.clone())
            } else {
                (rv, sp(loc, AutoRef(rv, Box::new(ty.clone()))))
            }
        }
        Anything | UnresolvedError | Unit | Param(_) | Apply(_, _, _) | Fun(_, _) => {
            (rv, sp(loc, AutoRef(rv, Box::new(ty))))
        }
        AutoRef(existing_rv, _) => (*existing_rv, ty),
    }
}

//**************************************************************************************************
// Structs
//**************************************************************************************************

pub fn make_struct_type(
    context: &mut Context,
    loc: Loc,
    m: &ModuleIdent,
    n: &StructName,
    ty_args_opt: Option<Vec<Type>>,
) -> (Type, Vec<Type>) {
    let tn = sp(loc, TypeName_::ModuleType(*m, *n));
    let sdef = context.struct_definition(m, n);
    match ty_args_opt {
        None => {
            let constraints = sdef
                .type_parameters
                .iter()
                .map(|tp| (loc, tp.param.abilities.clone()))
                .collect();
            let ty_args = make_tparams(context, loc, TVarCase::Base, constraints);
            (sp(loc, Type_::Apply(None, tn, ty_args.clone())), ty_args)
        }
        Some(ty_args) => {
            let tapply_ = instantiate_apply(context, loc, None, tn, ty_args);
            let targs = match &tapply_ {
                Type_::Apply(_, _, targs) => targs.clone(),
                _ => panic!("ICE instantiate_apply returned non Apply"),
            };
            (sp(loc, tapply_), targs)
        }
    }
}

pub fn make_expr_list_tvars(
    context: &mut Context,
    loc: Loc,
    constraint_msg: impl Into<String>,
    locs: Vec<Loc>,
) -> Vec<Type> {
    let constraints = locs.iter().map(|l| (*l, AbilitySet::empty())).collect();
    let tys = make_tparams(
        context,
        loc,
        TVarCase::Single(constraint_msg.into()),
        constraints,
    );
    tys.into_iter()
        .zip(locs)
        .map(|(tvar, l)| sp(l, tvar.value))
        .collect()
}

// ty_args should come from make_struct_type
pub fn make_field_types(
    context: &mut Context,
    _loc: Loc,
    m: &ModuleIdent,
    n: &StructName,
    ty_args: Vec<Type>,
) -> N::StructFields {
    let sdef = context.struct_definition(m, n);
    let tparam_subst = &make_tparam_subst(
        context
            .struct_definition(m, n)
            .type_parameters
            .iter()
            .map(|tp| &tp.param),
        ty_args,
    );
    match &sdef.fields {
        N::StructFields::Native(loc) => N::StructFields::Native(*loc),
        N::StructFields::Defined(m) => {
            N::StructFields::Defined(m.ref_map(|_, (idx, field_ty)| {
                (*idx, subst_tparams(tparam_subst, field_ty.clone()))
            }))
        }
    }
}

// ty_args should come from make_struct_type
pub fn make_field_type(
    context: &mut Context,
    loc: Loc,
    m: &ModuleIdent,
    n: &StructName,
    ty_args: Vec<Type>,
    field: &Field,
) -> Type {
    let sdef = context.struct_definition(m, n);
    let fields_map = match &sdef.fields {
        N::StructFields::Native(nloc) => {
            let nloc = *nloc;
            let msg = format!("Unbound field '{}' for native struct '{}::{}'", field, m, n);
            context.env.add_diag(diag!(
                NameResolution::UnboundField,
                (loc, msg),
                (nloc, "Struct declared 'native' here")
            ));
            return context.error_type(loc);
        }
        N::StructFields::Defined(m) => m,
    };
    match fields_map.get(field).cloned() {
        None => {
            context.env.add_diag(diag!(
                NameResolution::UnboundField,
                (loc, format!("Unbound field '{}' in '{}::{}'", field, m, n)),
            ));
            context.error_type(loc)
        }
        Some((_, field_ty)) => {
            let tparam_subst = &make_tparam_subst(
                context
                    .struct_definition(m, n)
                    .type_parameters
                    .iter()
                    .map(|tp| &tp.param),
                ty_args,
            );
            subst_tparams(tparam_subst, field_ty)
        }
    }
}

//**************************************************************************************************
// Constants
//**************************************************************************************************

pub fn make_constant_type(
    context: &mut Context,
    loc: Loc,
    m: &ModuleIdent,
    c: &ConstantName,
) -> Type {
    let in_current_module = Some(m) == context.current_module.as_ref();
    let (defined_loc, signature) = {
        let ConstantInfo {
            attributes: _,
            defined_loc,
            signature,
        } = context.constant_info(m, c);
        (*defined_loc, signature.clone())
    };
    if !in_current_module {
        let msg = format!("Invalid access of '{}::{}'", m, c);
        let internal_msg = "Constants are internal to their module, and cannot can be accessed \
                            outside of their module";
        context.env.add_diag(diag!(
            TypeSafety::Visibility,
            (loc, msg),
            (defined_loc, internal_msg)
        ));
    }

    signature
}

//**************************************************************************************************
// Functions
//**************************************************************************************************

pub fn make_method_call_type(
    context: &mut Context,
    loc: Loc,
    lhs_ty: &Type,
    tn: &TypeName,
    method: Name,
    ty_args_opt: Option<Vec<Type>>,
) -> Option<(ModuleIdent, FunctionName, ResolvedFunctionType)> {
    let target_function_opt = context.find_method_and_mark_used(tn, method);
    // try to find a function in the defining module for errors
    let Some((target_m, target_f)) = target_function_opt else {
        let lhs_ty_str = error_format_nested(lhs_ty, &context.subst);
        let defining_module = match &tn.value {
            TypeName_::Multiple(_) => panic!("ICE method on tuple"),
            TypeName_::Builtin(sp!(_, bt_)) => context.env.primitive_definer(*bt_),
            TypeName_::ModuleType(m, _) => Some(m),
        };
        let finfo_opt = defining_module.and_then(|m| {
            let finfo = context
                .modules
                .module(m)
                .functions
                .get(&FunctionName(method))?;
            Some((m, finfo))
        });
        // if we found a function with the method name, it must have the wrong type
        if let Some((m, finfo)) = finfo_opt {
            let (first_ty_loc, first_ty) = match finfo
                .signature
                .parameters
                .first()
                .map(|(_, _, t)| t.clone())
            {
                None => (finfo.defined_loc, None),
                Some(t) => (t.loc, Some(t)),
            };
            let arg_msg = match first_ty {
                Some(ty) => {
                    let tys_str = error_format(&ty, &context.subst);
                    format!("but it has a different type for its first argument, {tys_str}")
                }
                None => "but it takes no arguments".to_owned(),
            };
            let msg = format!(
                "Invalid method call. \
                No known method '{method}' on type '{lhs_ty_str}'"
            );
            let fmsg = format!("The function '{m}::{method}' exists, {arg_msg}");
            context.env.add_diag(diag!(
                TypeSafety::InvalidMethodCall,
                (loc, msg),
                (first_ty_loc, fmsg)
            ));
        } else {
            let msg = format!(
                "Invalid method call. \
                No known method '{method}' on type '{lhs_ty_str}'"
            );
            let decl_msg = match defining_module {
                Some(m) => {
                    format!(", and no function '{method}' was found in the defining module '{m}'")
                }
                None => "".to_owned(),
            };
            let fmsg =
                format!("No local 'use fun' alias was found for '{lhs_ty_str}.{method}'{decl_msg}");
            context.env.add_diag(diag!(
                TypeSafety::InvalidMethodCall,
                (loc, msg),
                (method.loc, fmsg)
            ));
        }
        return None;
    };

    let function_ty = make_function_type(context, loc, &target_m, &target_f, ty_args_opt);

    Some((target_m, target_f, function_ty))
}

pub fn make_function_type(
    context: &mut Context,
    loc: Loc,
    m: &ModuleIdent,
    f: &FunctionName,
    ty_args_opt: Option<Vec<Type>>,
) -> ResolvedFunctionType {
    let in_current_module = match &context.current_module {
        Some(current) => m == current,
        None => false,
    };
    let finfo = context.function_info(m, f);
    let macro_ = finfo.macro_;
    let constraints: Vec<_> = finfo
        .signature
        .type_parameters
        .iter()
        .map(|tp| tp.abilities.clone())
        .collect();

    let ty_args = match ty_args_opt {
        None => {
            let case = if macro_.is_some() {
                TVarCase::Macro
            } else {
                TVarCase::Base
            };
            let locs_constraints = constraints.into_iter().map(|k| (loc, k)).collect();
            make_tparams(context, loc, case, locs_constraints)
        }
        Some(ty_args) => {
            let case = if macro_.is_some() {
                TArgCase::Macro
            } else {
                TArgCase::Fun
            };
            let ty_args = check_type_argument_arity(
                context,
                loc,
                || format!("{}::{}", m, f),
                ty_args,
                &constraints,
            );
            instantiate_type_args(context, loc, case, ty_args, constraints)
        }
    };

    let finfo = context.function_info(m, f);
    let tparam_subst = &make_tparam_subst(&finfo.signature.type_parameters, ty_args.clone());
    let params = finfo
        .signature
        .parameters
        .iter()
        .map(|(_, n, t)| (*n, subst_tparams(tparam_subst, t.clone())))
        .collect();
    let return_ty = subst_tparams(tparam_subst, finfo.signature.return_type.clone());

    let defined_loc = finfo.defined_loc;
    let public_for_testing =
        public_testing_visibility(context.env, context.current_package, f, finfo.entry);
    let is_testing_context = context.is_testing_context();
    match finfo.visibility {
        _ if is_testing_context && public_for_testing.is_some() => (),
        Visibility::Internal if in_current_module => (),
        Visibility::Internal => {
            let internal_msg = format!(
                "This function is internal to its module. Only '{}', '{}', and '{}' functions can \
                 be called outside of their module",
                Visibility::PUBLIC,
                Visibility::FRIEND,
                Visibility::PACKAGE
            );
            visibility_error(
                context,
                public_for_testing,
                (loc, format!("Invalid call to internal function '{m}::{f}'")),
                (defined_loc, internal_msg),
            );
        }
        Visibility::Package(loc)
            if in_current_module || context.current_module_shares_package_and_address(m) =>
        {
            context.record_current_module_as_friend(m, loc);
        }
        Visibility::Package(vis_loc) => {
            let msg = format!(
                "Invalid call to '{}' visible function '{}::{}'",
                Visibility::PACKAGE,
                m,
                f
            );
            let internal_msg = format!(
                "A '{}' function can only be called from the same address and package as \
                module '{}' in package '{}'. This call is from address '{}' in package '{}'",
                Visibility::PACKAGE,
                m,
                context
                    .module_info(m)
                    .package
                    .map(|pkg_name| format!("{}", pkg_name))
                    .unwrap_or("<unknown package>".to_string()),
                &context
                    .current_module
                    .map(|cur_module| cur_module.value.address.to_string())
                    .unwrap_or("<unknown addr>".to_string()),
                &context
                    .current_module
                    .and_then(|cur_module| context.module_info(&cur_module).package)
                    .map(|pkg_name| format!("{}", pkg_name))
                    .unwrap_or("<unknown package>".to_string())
            );
            visibility_error(
                context,
                public_for_testing,
                (loc, msg),
                (vis_loc, internal_msg),
            );
        }
        Visibility::Friend(_) if in_current_module || context.current_module_is_a_friend_of(m) => {}
        Visibility::Friend(vis_loc) => {
            let msg = format!(
                "Invalid call to '{}' visible function '{m}::{f}'",
                Visibility::FRIEND,
            );
            let internal_msg =
                format!("This function can only be called from a 'friend' of module '{m}'",);
            visibility_error(
                context,
                public_for_testing,
                (loc, msg),
                (vis_loc, internal_msg),
            );
        }
        Visibility::Public(_) => (),
    };
    ResolvedFunctionType {
        declared: defined_loc,
        macro_,
        ty_args,
        params,
        return_: return_ty,
    }
}

#[derive(Clone, Copy)]
pub enum PublicForTesting {
    /// The function is entry, so it can be called in unit tests
    Entry(Loc),
    // TODO we should allow calling init in unit tests, but this would need Sui bytecode verifier
    // support. Or we would need to name dodge init in unit tests
    // SuiInit(Loc),
}

pub fn public_testing_visibility(
    env: &CompilationEnv,
    _package: Option<Symbol>,
    _callee_name: &FunctionName,
    callee_entry: Option<Loc>,
) -> Option<PublicForTesting> {
    // is_testing && (is_entry || is_sui_init)
    if !env.flags().is_testing() {
        return None;
    }

    // TODO support sui init functions
    // let flavor = env.package_config(package).flavor;
    // flavor == Flavor::Sui && callee_name.value() == INIT_FUNCTION_NAME
    callee_entry.map(PublicForTesting::Entry)
}

fn visibility_error(
    context: &mut Context,
    public_for_testing: Option<PublicForTesting>,
    (call_loc, call_msg): (Loc, impl ToString),
    (vis_loc, vis_msg): (Loc, impl ToString),
) {
    let mut diag = diag!(
        TypeSafety::Visibility,
        (call_loc, call_msg),
        (vis_loc, vis_msg),
    );
    if context.env.flags().is_testing() {
        if let Some(case) = public_for_testing {
            let (test_loc, test_msg) = match case {
                PublicForTesting::Entry(entry_loc) => {
                    let entry_msg = format!(
                        "'{}' functions can be called in tests, \
                    but only from testing contexts, e.g. '#[{}]' or '#[{}]'",
                        ENTRY_MODIFIER,
                        TestingAttribute::TEST,
                        TestingAttribute::TEST_ONLY,
                    );
                    (entry_loc, entry_msg)
                }
            };
            diag.add_secondary_label((test_loc, test_msg))
        }
    }
    context.env.add_diag(diag)
}

pub fn check_call_arity<S: std::fmt::Display, F: Fn() -> S>(
    context: &mut Context,
    loc: Loc,
    msg: F,
    arity: usize,
    argloc: Loc,
    given_len: usize,
) {
    if given_len == arity {
        return;
    }
    let code = if given_len < arity {
        TypeSafety::TooFewArguments
    } else {
        TypeSafety::TooManyArguments
    };
    let cmsg = format!(
        "{}. The call expected {} argument(s) but got {}",
        msg(),
        arity,
        given_len
    );
    context.env.add_diag(diag!(
        code,
        (loc, cmsg),
        (argloc, format!("Found {} argument(s) here", given_len)),
    ));
}

//**************************************************************************************************
// Constraints
//**************************************************************************************************

pub fn solve_constraints(context: &mut Context) {
    use BuiltinTypeName_ as BT;
    let num_vars = context.subst.num_vars.clone();
    let mut subst = std::mem::replace(&mut context.subst, Subst::empty());
    for (num_var, loc) in num_vars {
        let tvar = sp(loc, Type_::Var(num_var));
        match unfold_type(&subst, tvar.clone()).value {
            Type_::UnresolvedError | Type_::Anything => {
                let next_subst = join(subst, &Type_::u64(loc), &tvar).unwrap().0;
                subst = next_subst;
            }
            _ => (),
        }
    }
    context.subst = subst;

    let constraints = std::mem::take(&mut context.constraints);
    for constraint in constraints {
        match constraint {
            Constraint::AbilityConstraint {
                loc,
                msg,
                ty,
                constraints,
            } => solve_ability_constraint(context, loc, msg, ty, constraints),
            Constraint::NumericConstraint(loc, op, t) => {
                solve_builtin_type_constraint(context, BT::numeric(), loc, op, t)
            }
            Constraint::BitsConstraint(loc, op, t) => {
                solve_builtin_type_constraint(context, BT::bits(), loc, op, t)
            }
            Constraint::OrderedConstraint(loc, op, t) => {
                solve_builtin_type_constraint(context, BT::ordered(), loc, op, t)
            }
            Constraint::BaseTypeConstraint(loc, msg, t) => {
                solve_base_type_constraint(context, loc, msg, &t)
            }
            Constraint::SingleTypeConstraint(loc, msg, t) => {
                solve_single_type_constraint(context, loc, msg, &t)
            }
        }
    }
}

fn solve_ability_constraint(
    context: &mut Context,
    loc: Loc,
    given_msg_opt: Option<String>,
    ty: Type,
    constraints: AbilitySet,
) {
    let ty = unfold_type(&context.subst, ty);
    let ty_abilities = infer_abilities(&context.modules, &context.subst, ty.clone());

    let (declared_loc_opt, declared_abilities, ty_args) = debug_abilities_info(context, &ty);
    for constraint in constraints {
        if ty_abilities.has_ability(&constraint) {
            continue;
        }

        let constraint_msg = match &given_msg_opt {
            Some(s) => s.clone(),
            None => format!("'{}' constraint not satisifed", constraint),
        };
        let mut diag = diag!(AbilitySafety::Constraint, (loc, constraint_msg));
        ability_not_satisfied_tips(
            &context.subst,
            &mut diag,
            constraint.value,
            &ty,
            declared_loc_opt,
            &declared_abilities,
            ty_args.iter().map(|ty_arg| {
                let abilities = infer_abilities(&context.modules, &context.subst, ty_arg.clone());
                (ty_arg, abilities)
            }),
        );

        // is none if it is from a user constraint and not a part of the type system
        if given_msg_opt.is_none() {
            diag.add_secondary_label((
                constraint.loc,
                format!("'{}' constraint declared here", constraint),
            ));
        }
        context.env.add_diag(diag)
    }
}

pub fn ability_not_satisfied_tips<'a>(
    subst: &Subst,
    diag: &mut Diagnostic,
    constraint: Ability_,
    ty: &Type,
    declared_loc_opt: Option<Loc>,
    declared_abilities: &AbilitySet,
    ty_args: impl IntoIterator<Item = (&'a Type, AbilitySet)>,
) {
    let ty_str = error_format(ty, subst);
    let ty_msg = format!(
        "The type {} does not have the ability '{}'",
        ty_str, constraint
    );
    diag.add_secondary_label((ty.loc, ty_msg));
    match (
        declared_loc_opt,
        declared_abilities.has_ability_(constraint),
    ) {
        // Type was not given the ability
        (Some(dloc), false) => diag.add_secondary_label((
            dloc,
            format!(
                "To satisfy the constraint, the '{}' ability would need to be added here",
                constraint
            ),
        )),
        // Type does not have the ability
        (_, false) => (),
        // Type has the ability but a type argument causes it to fail
        (_, true) => {
            let requirement = constraint.requires();
            let mut label_added = false;
            for (ty_arg, ty_arg_abilities) in ty_args {
                if !ty_arg_abilities.has_ability_(requirement) {
                    let ty_arg_str = error_format(ty_arg, subst);
                    let msg = format!(
                        "The type {ty} can have the ability '{constraint}' but the type argument \
                         {ty_arg} does not have the required ability '{requirement}'",
                        ty = ty_str,
                        ty_arg = ty_arg_str,
                        constraint = constraint,
                        requirement = requirement,
                    );
                    diag.add_secondary_label((ty_arg.loc, msg));
                    label_added = true;
                    break;
                }
            }
            assert!(label_added)
        }
    }
}

fn solve_builtin_type_constraint(
    context: &mut Context,
    builtin_set: &BTreeSet<BuiltinTypeName_>,
    loc: Loc,
    op: &'static str,
    ty: Type,
) {
    use TypeName_::*;
    use Type_::*;
    let t = unfold_type(&context.subst, ty);
    let tloc = t.loc;
    let mk_tmsg = || {
        let set_msg = if builtin_set.is_empty() {
            "the operation is not yet supported on any type".to_string()
        } else {
            format!(
                "expected: {}",
                format_comma(builtin_set.iter().map(|b| format!("'{}'", b)))
            )
        };
        format!(
            "Found: {}. But {}",
            error_format(&t, &context.subst),
            set_msg
        )
    };
    match &t.value {
        // already failed, ignore
        UnresolvedError => (),
        // Will fail later in compiling, either through dead code, or unknown type variable
        Anything => (),
        Apply(abilities_opt, sp!(_, Builtin(sp!(_, b))), args) if builtin_set.contains(b) => {
            if let Some(abilities) = abilities_opt {
                assert!(
                    abilities.has_ability_(Ability_::Drop),
                    "ICE assumes this type is being consumed so should have drop"
                );
            }
            assert!(args.is_empty());
        }
        _ => {
            let tmsg = mk_tmsg();
            context.env.add_diag(diag!(
                TypeSafety::BuiltinOperation,
                (loc, format!("Invalid argument to '{}'", op)),
                (tloc, tmsg)
            ))
        }
    }
}

fn solve_base_type_constraint(context: &mut Context, loc: Loc, msg: String, ty: &Type) {
    use TypeName_::*;
    use Type_::*;
    let sp!(tyloc, unfolded_) = unfold_type(&context.subst, ty.clone());
    match unfolded_ {
        Var(_) | AutoRef(_, _) => unreachable!(),
        Unit | Ref(_, _) | Apply(_, sp!(_, Multiple(_)), _) => {
            let tystr = error_format(ty, &context.subst);
            let tmsg = format!("Expected a single non-reference type, but found: {}", tystr);
            context.env.add_diag(diag!(
                TypeSafety::ExpectedBaseType,
                (loc, msg),
                (tyloc, tmsg)
            ))
        }
        UnresolvedError | Anything | Param(_) | Apply(_, _, _) | Fun(_, _) => (),
    }
}

fn solve_single_type_constraint(context: &mut Context, loc: Loc, msg: String, ty: &Type) {
    use TypeName_::*;
    use Type_::*;
    let sp!(tyloc, unfolded_) = unfold_type(&context.subst, ty.clone());
    match unfolded_ {
        Var(_) | AutoRef(_, _) => unreachable!(),
        Unit | Apply(_, sp!(_, Multiple(_)), _) => {
            let tmsg = format!(
                "Expected a single type, but found expression list type: {}",
                error_format(ty, &context.subst)
            );
            context.env.add_diag(diag!(
                TypeSafety::ExpectedSingleType,
                (loc, msg),
                (tyloc, tmsg)
            ))
        }
        UnresolvedError | Anything | Ref(_, _) | Param(_) | Apply(_, _, _) | Fun(_, _) => (),
    }
}

//**************************************************************************************************
// Subst
//**************************************************************************************************

pub fn unfold_type(subst: &Subst, sp!(loc, t_): Type) -> Type {
    match t_ {
        Type_::Var(i) => {
            let last_tvar = forward_tvar(subst, i);
            match subst.get(last_tvar) {
                Some(sp!(_, Type_::Var(_))) => unreachable!(),
                None => sp(loc, Type_::Anything),
                Some(inner) => inner.clone(),
            }
        }
        Type_::AutoRef(id, t) => {
            let x = forward_ref_var(subst, id);
            let t = Box::new(unfold_type(subst, *t));
            match subst.get_ref_var(x) {
                None => panic!("ICE unresolved autoref"),
                Some(ref_kind) => match ref_kind {
                    RefKind::Forward(_) => unreachable!(),
                    RefKind::Value => *t,
                    RefKind::ImmRef => sp(loc, Type_::Ref(false, t)),
                    RefKind::MutRef => sp(loc, Type_::Ref(true, t)),
                },
            }
        }
        x => sp(loc, x),
    }
}

// Equivelent to unfold_type, but only returns the loc.
// The hope is to point to the last loc in a chain of type var's, giving the loc closest to the
// actual type in the source code
pub fn best_loc(subst: &Subst, sp!(loc, t_): &Type) -> Loc {
    match t_ {
        Type_::Var(i) => {
            let last_tvar = forward_tvar(subst, *i);
            match subst.get(last_tvar) {
                Some(sp!(_, Type_::Var(_))) => unreachable!(),
                None => *loc,
                Some(sp!(inner_loc, _)) => *inner_loc,
            }
        }
        _ => *loc,
    }
}

pub fn make_tparam_subst<'a, I1, I2>(tps: I1, args: I2) -> TParamSubst
where
    I1: IntoIterator<Item = &'a TParam>,
    I1::IntoIter: ExactSizeIterator,
    I2: IntoIterator<Item = Type>,
    I2::IntoIter: ExactSizeIterator,
{
    let tps = tps.into_iter();
    let args = args.into_iter();
    assert!(tps.len() == args.len());
    let mut subst = TParamSubst::new();
    for (tp, arg) in tps.zip(args) {
        let old_val = subst.insert(tp.id, arg);
        assert!(old_val.is_none())
    }
    subst
}

pub fn subst_tparams(subst: &TParamSubst, sp!(loc, t_): Type) -> Type {
    use Type_::*;
    match t_ {
        x @ Unit | x @ UnresolvedError | x @ Anything => sp(loc, x),
        Var(_) => panic!("ICE tvar in subst_tparams"),
        AutoRef(_, _) => panic!("ICE autoref in subst_tparams"),
        Ref(mut_, t) => sp(loc, Ref(mut_, Box::new(subst_tparams(subst, *t)))),
        Param(tp) => subst
            .get(&tp.id)
            .expect("ICE unmapped tparam in subst_tparams_base")
            .clone(),
        Apply(k, n, ty_args) => {
            let ftys = ty_args
                .into_iter()
                .map(|t| subst_tparams(subst, t))
                .collect();
            sp(loc, Apply(k, n, ftys))
        }
        Fun(args, result) => {
            let ftys = args.into_iter().map(|t| subst_tparams(subst, t)).collect();
            let fres = Box::new(subst_tparams(subst, *result));
            sp(loc, Fun(ftys, fres))
        }
    }
}

pub fn ready_tvars(subst: &Subst, sp!(loc, t_): Type) -> Type {
    use Type_::*;
    match t_ {
        x @ UnresolvedError | x @ Unit | x @ Anything | x @ Param(_) => sp(loc, x),
        Ref(mut_, t) => sp(loc, Ref(mut_, Box::new(ready_tvars(subst, *t)))),
        Apply(k, n, tys) => {
            let tys = tys.into_iter().map(|t| ready_tvars(subst, t)).collect();
            sp(loc, Apply(k, n, tys))
        }
        Var(i) => {
            let last_var = forward_tvar(subst, i);
            match subst.get(last_var) {
                Some(sp!(_, Var(_))) => unreachable!(),
                None => sp(loc, Var(last_var)),
                Some(t) => ready_tvars(subst, t.clone()),
            }
        }
        Fun(args, result) => {
            let args = args.into_iter().map(|t| ready_tvars(subst, t)).collect();
            let result = Box::new(ready_tvars(subst, *result));
            sp(loc, Fun(args, result))
        }
        AutoRef(id, t) => {
            let x = forward_ref_var(subst, id);
            let t = Box::new(ready_tvars(subst, *t));
            match subst.get_ref_var(x) {
                None => sp(loc, AutoRef(x, t)),
                Some(ref_kind) => match ref_kind {
                    RefKind::Forward(_) => unreachable!(),
                    RefKind::Value => *t,
                    RefKind::ImmRef => sp(loc, Ref(false, t)),
                    RefKind::MutRef => sp(loc, Ref(true, t)),
                },
            }
        }
    }
}

pub fn unfold_ref_var(subst: &Subst, id: RefVar) -> Option<&RefKind> {
    let rv = forward_ref_var(subst, id);
    subst.get_ref_var(rv)
}

//**************************************************************************************************
// Instantiate
//**************************************************************************************************

pub fn instantiate(context: &mut Context, sp!(loc, t_): Type) -> Type {
    use Type_::*;
    let it_ = match t_ {
        Unit => Unit,
        UnresolvedError => UnresolvedError,
        Anything => make_tvar(context, loc).value,
        Ref(mut_, b) => {
            let inner = *b;
            context.add_base_type_constraint(loc, "Invalid reference type", inner.clone());
            Ref(mut_, Box::new(instantiate(context, inner)))
        }
        Apply(abilities_opt, n, ty_args) => {
            instantiate_apply(context, loc, abilities_opt, n, ty_args)
        }
        Fun(args, result) => Fun(
            args.into_iter().map(|t| instantiate(context, t)).collect(),
            Box::new(instantiate(context, *result)),
        ),
        x @ Param(_) => x,
        // instantiating a var really shouldn't happen... but it does because of macro expansion
        // We expand macros before type checking, but after the arguments to the macro are type
        // checked (otherwise we couldn't properly do method syntax macros). As a result, we are
        // substituting type variables into the macro body, and might hit one while expanding a
        // type in the macro where a type parameter's argument had a type variable.
        x @ Var(_) => x,
        // Conversely, autorefs should not live long enough to flow to an instantiation spot.
        AutoRef(_, _) => panic!("ICE instantiate autoref"),
    };
    sp(loc, it_)
}

// abilities_opt is expected to be None for non primitive types
fn instantiate_apply(
    context: &mut Context,
    loc: Loc,
    abilities_opt: Option<AbilitySet>,
    n: TypeName,
    ty_args: Vec<Type>,
) -> Type_ {
    let tparam_constraints: Vec<AbilitySet> = match &n {
        sp!(nloc, N::TypeName_::Builtin(b)) => b.value.tparam_constraints(*nloc),
        sp!(_, N::TypeName_::Multiple(len)) => {
            debug_assert!(abilities_opt.is_none(), "ICE instantiated expanded type");
            (0..*len).map(|_| AbilitySet::empty()).collect()
        }
        sp!(_, N::TypeName_::ModuleType(m, s)) => {
            debug_assert!(abilities_opt.is_none(), "ICE instantiated expanded type");
            let tps = context.struct_tparams(m, s);
            tps.iter().map(|tp| tp.param.abilities.clone()).collect()
        }
    };

    let tys = instantiate_type_args(
        context,
        loc,
        TArgCase::Apply(&n.value),
        ty_args,
        tparam_constraints,
    );
    Type_::Apply(abilities_opt, n, tys)
}

// The type arguments are bound to type variables after intantiation
// i.e. vec<t1, ..., tn> ~> vec<a1, ..., an> s.t a1 => t1, ... , an => tn
// This might be needed for any variance case, and I THINK that it should be fine without it
// BUT I'm adding it as a safeguard against instantiating twice. Can always remove once this
// stabilizes
fn instantiate_type_args(
    context: &mut Context,
    loc: Loc,
    case: TArgCase,
    mut ty_args: Vec<Type>,
    constraints: Vec<AbilitySet>,
) -> Vec<Type> {
    assert!(ty_args.len() == constraints.len());
    let locs_constraints = constraints
        .into_iter()
        .zip(&ty_args)
        .map(|(abilities, t)| (t.loc, abilities))
        .collect();
    let tvar_case = match case {
        TArgCase::Apply(TypeName_::Multiple(_)) => {
            TVarCase::Single("Invalid expression list type argument".to_owned())
        }
        TArgCase::Fun
        | TArgCase::Apply(TypeName_::Builtin(_))
        | TArgCase::Apply(TypeName_::ModuleType(_, _)) => TVarCase::Base,
        TArgCase::Macro => TVarCase::Macro,
    };
    let tvars = make_tparams(context, loc, tvar_case, locs_constraints);
    ty_args = ty_args
        .into_iter()
        .map(|t| instantiate(context, t))
        .collect();

    assert!(ty_args.len() == tvars.len());
    let mut res = vec![];
    let subst = std::mem::replace(&mut context.subst, /* dummy value */ Subst::empty());
    context.subst = tvars
        .into_iter()
        .zip(ty_args)
        .fold(subst, |subst, (tvar, ty_arg)| {
            // tvar is just a type variable, so shouldn't throw ever...
            let (subst, t) = join(subst, &tvar, &ty_arg).ok().unwrap();
            res.push(t);
            subst
        });
    res
}

fn check_type_argument_arity<F: FnOnce() -> String>(
    context: &mut Context,
    loc: Loc,
    name_f: F,
    mut ty_args: Vec<Type>,
    tparam_constraints: &[AbilitySet],
) -> Vec<Type> {
    let args_len = ty_args.len();
    let arity = tparam_constraints.len();
    if args_len != arity {
        let code = if args_len < arity {
            NameResolution::TooFewTypeArguments
        } else {
            NameResolution::TooManyTypeArguments
        };
        let msg = format!(
            "Invalid instantiation of '{}'. Expected {} type argument(s) but got {}",
            name_f(),
            arity,
            args_len
        );
        context.env.add_diag(diag!(code, (loc, msg)));
    }

    while ty_args.len() > arity {
        ty_args.pop();
    }

    while ty_args.len() < arity {
        ty_args.push(context.error_type(loc));
    }

    ty_args
}

enum TVarCase {
    Single(String),
    Base,
    Macro,
}

enum TArgCase<'a> {
    Apply(&'a TypeName_),
    Fun,
    Macro,
}

fn make_tparams(
    context: &mut Context,
    loc: Loc,
    case: TVarCase,
    tparam_constraints: Vec<(Loc, AbilitySet)>,
) -> Vec<Type> {
    tparam_constraints
        .into_iter()
        .map(|(vloc, constraint)| {
            let tvar = make_tvar(context, vloc);
            context.add_ability_set_constraint(loc, None::<String>, tvar.clone(), constraint);
            match &case {
                TVarCase::Single(msg) => context.add_single_type_constraint(loc, msg, tvar.clone()),
                TVarCase::Base => {
                    context.add_base_type_constraint(loc, "Invalid type argument", tvar.clone())
                }
                TVarCase::Macro => (),
            };
            tvar
        })
        .collect()
}

// used in macros to make the signatures consistent with the bodies, in that we don't check
// constraints until application
pub fn give_tparams_all_abilities(sp!(_, ty_): &mut Type) {
    match ty_ {
        Type_::Unit | Type_::Var(_) | Type_::UnresolvedError | Type_::Anything => (),
        Type_::Ref(_, inner) => give_tparams_all_abilities(inner),
        Type_::Apply(_, _, ty_args) => {
            for ty_arg in ty_args {
                give_tparams_all_abilities(ty_arg)
            }
        }
        Type_::Fun(args, ret) => {
            for arg in args {
                give_tparams_all_abilities(arg)
            }
            give_tparams_all_abilities(ret)
        }
        Type_::Param(_) => *ty_ = Type_::Anything,
        Type_::AutoRef(_, _) => panic!("ICE autoref should never occur in macro signature"),
    }
}

//**************************************************************************************************
// Subtype and joining
//**************************************************************************************************

#[derive(Debug)]
pub enum TypingError {
    SubtypeError(Box<Type>, Box<Type>),
    Incompatible(Box<Type>, Box<Type>),
    ArityMismatch(usize, Box<Type>, usize, Box<Type>),
    FunArityMismatch(usize, Box<Type>, usize, Box<Type>),
    RecursiveType(Loc),
}

#[derive(Clone, Copy, Debug)]
enum TypingCase {
    Join,
    Subtype,
}

pub fn subtype(subst: Subst, lhs: &Type, rhs: &Type) -> Result<(Subst, Type), TypingError> {
    join_impl(subst, TypingCase::Subtype, lhs, rhs)
}

pub fn join(subst: Subst, lhs: &Type, rhs: &Type) -> Result<(Subst, Type), TypingError> {
    join_impl(subst, TypingCase::Join, lhs, rhs)
}

fn join_impl(
    mut subst: Subst,
    case: TypingCase,
    lhs: &Type,
    rhs: &Type,
) -> Result<(Subst, Type), TypingError> {
    use TypeName_::*;
    use Type_::*;
    use TypingCase::*;
    match (lhs, rhs) {
        (sp!(_, Anything), other) | (other, sp!(_, Anything)) => Ok((subst, other.clone())),

        (sp!(_, Unit), sp!(loc, Unit)) => Ok((subst, sp(*loc, Unit))),

        (sp!(loc1, Ref(mut1, t1)), sp!(loc2, Ref(mut2, t2))) => {
            let (loc, mut_) = match (case, mut1, mut2) {
                (Join, _, _) => {
                    // if 1 is imm and 2 is mut, use loc1. Else, loc2
                    let loc = if !*mut1 && *mut2 { *loc1 } else { *loc2 };
                    (loc, *mut1 && *mut2)
                }
                // imm <: imm
                // mut <: imm
                (Subtype, false, false) | (Subtype, true, false) => (*loc2, false),
                // mut <: mut
                (Subtype, true, true) => (*loc2, true),
                // imm <\: mut
                (Subtype, false, true) => {
                    return Err(TypingError::SubtypeError(
                        Box::new(lhs.clone()),
                        Box::new(rhs.clone()),
                    ))
                }
            };
            let (subst, t) = join_impl(subst, case, t1, t2)?;
            Ok((subst, sp(loc, Ref(mut_, Box::new(t)))))
        }
        (sp!(_, Param(TParam { id: id1, .. })), sp!(_, Param(TParam { id: id2, .. })))
            if id1 == id2 =>
        {
            Ok((subst, rhs.clone()))
        }
        (sp!(_, Apply(_, sp!(_, Multiple(n1)), _)), sp!(_, Apply(_, sp!(_, Multiple(n2)), _)))
            if n1 != n2 =>
        {
            Err(TypingError::ArityMismatch(
                *n1,
                Box::new(lhs.clone()),
                *n2,
                Box::new(rhs.clone()),
            ))
        }
        (sp!(_, Apply(k1, n1, tys1)), sp!(loc, Apply(k2, n2, tys2))) if n1 == n2 => {
            assert!(
                k1 == k2,
                "ICE failed naming: {:#?}kind != {:#?}kind. {:#?} !=  {:#?}",
                n1,
                n2,
                k1,
                k2
            );
            let (subst, tys) = join_impl_types(subst, case, tys1, tys2)?;
            Ok((subst, sp(*loc, Apply(k2.clone(), n2.clone(), tys))))
        }
        (sp!(_, Fun(a1, _)), sp!(_, Fun(a2, _))) if a1.len() != a2.len() => {
            Err(TypingError::FunArityMismatch(
                a1.len(),
                Box::new(lhs.clone()),
                a2.len(),
                Box::new(rhs.clone()),
            ))
        }
        (sp!(_, Fun(a1, r1)), sp!(loc, Fun(a2, r2))) => {
            // TODO this is going to likely lead to some strange error locations/messages
            // since the RHS in subtyping is currently assumed to be an annotation
            let (subst, args) = match case {
                Join => join_impl_types(subst, case, a1, a2)?,
                Subtype => join_impl_types(subst, case, a2, a1)?,
            };
            let (subst, result) = join_impl(subst, case, r1, r2)?;
            Ok((subst, sp(*loc, Fun(args, Box::new(result)))))
        }
        (sp!(loc1, Var(id1)), sp!(loc2, Var(id2))) => {
            if *id1 == *id2 {
                Ok((subst, sp(*loc2, Var(*id2))))
            } else {
                join_tvar(subst, case, *loc1, *id1, *loc2, *id2)
            }
        }
        (sp!(loc, Var(id)), other) if subst.get(*id).is_none() => {
            if join_bind_tvar(&mut subst, *loc, *id, other.clone())? {
                Ok((subst, sp(*loc, Var(*id))))
            } else {
                Err(TypingError::Incompatible(
                    Box::new(sp(*loc, Var(*id))),
                    Box::new(other.clone()),
                ))
            }
        }
        (other, sp!(loc, Var(id))) if subst.get(*id).is_none() => {
            if join_bind_tvar(&mut subst, *loc, *id, other.clone())? {
                Ok((subst, sp(*loc, Var(*id))))
            } else {
                Err(TypingError::Incompatible(
                    Box::new(other.clone()),
                    Box::new(sp(*loc, Var(*id))),
                ))
            }
        }
        (sp!(loc, Var(id)), other) => {
            let new_tvar = TVar::next();
            subst.insert(new_tvar, other.clone());
            join_tvar(subst, case, *loc, *id, other.loc, new_tvar)
        }
        (other, sp!(loc, Var(id))) => {
            let new_tvar = TVar::next();
            subst.insert(new_tvar, other.clone());
            join_tvar(subst, case, other.loc, new_tvar, *loc, *id)
        }
        (sp!(loc1, AutoRef(rv1, t1)), sp!(loc2, AutoRef(rv2, t2))) => {
            join_autoref(subst, case, *loc1, *rv1, t1, *loc2, *rv2, t2)
        }
        (other, sp!(loc, AutoRef(rv, rvtype))) | (sp!(loc, AutoRef(rv, rvtype)), other) => {
            join_bind_autoref(subst, case, *loc, *rv, rvtype, other)
        }
        (sp!(_, UnresolvedError), other) | (other, sp!(_, UnresolvedError)) => {
            Ok((subst, other.clone()))
        }
        _ => Err(TypingError::Incompatible(
            Box::new(lhs.clone()),
            Box::new(rhs.clone()),
        )),
    }
}

fn join_impl_types(
    mut subst: Subst,
    case: TypingCase,
    tys1: &[Type],
    tys2: &[Type],
) -> Result<(Subst, Vec<Type>), TypingError> {
    // if tys1.len() != tys2.len(), we will get an error when instantiating the type elsewhere
    // as all types are instantiated as a sanity check
    let mut tys = vec![];
    for (ty1, ty2) in tys1.iter().zip(tys2) {
        let (nsubst, t) = join_impl(subst, case, ty1, ty2)?;
        subst = nsubst;
        tys.push(t)
    }
    Ok((subst, tys))
}

fn join_tvar(
    mut subst: Subst,
    case: TypingCase,
    loc1: Loc,
    id1: TVar,
    loc2: Loc,
    id2: TVar,
) -> Result<(Subst, Type), TypingError> {
    use Type_::*;
    let last_id1 = forward_tvar(&subst, id1);
    let last_id2 = forward_tvar(&subst, id2);
    let ty1 = match subst.get(last_id1) {
        None => sp(loc1, Anything),
        Some(t) => t.clone(),
    };
    let ty2 = match subst.get(last_id2) {
        None => sp(loc2, Anything),
        Some(t) => t.clone(),
    };

    let new_tvar = TVar::next();
    let num_loc_1 = subst.num_vars.get(&last_id1);
    let num_loc_2 = subst.num_vars.get(&last_id2);
    match (num_loc_1, num_loc_2) {
        (_, Some(nloc)) | (Some(nloc), _) => {
            let nloc = *nloc;
            subst.set_num_var(new_tvar, nloc);
        }
        _ => (),
    }
    subst.insert(last_id1, sp(loc1, Var(new_tvar)));
    subst.insert(last_id2, sp(loc2, Var(new_tvar)));

    let (mut subst, new_ty) = join_impl(subst, case, &ty1, &ty2)?;
    match subst.get(new_tvar) {
        Some(sp!(tloc, _)) => Err(TypingError::RecursiveType(*tloc)),
        None => {
            if join_bind_tvar(&mut subst, loc2, new_tvar, new_ty)? {
                Ok((subst, sp(loc2, Var(new_tvar))))
            } else {
                let ty1 = match ty1 {
                    sp!(loc, Anything) => sp(loc, Var(id1)),
                    t => t,
                };
                let ty2 = match ty2 {
                    sp!(loc, Anything) => sp(loc, Var(id2)),
                    t => t,
                };
                Err(TypingError::Incompatible(Box::new(ty1), Box::new(ty2)))
            }
        }
    }
}

fn forward_tvar(subst: &Subst, id: TVar) -> TVar {
    let mut cur = id;
    loop {
        match subst.get(cur) {
            Some(sp!(_, Type_::Var(next))) => cur = *next,
            Some(_) | None => break cur,
        }
    }
}

fn join_bind_tvar(subst: &mut Subst, loc: Loc, tvar: TVar, ty: Type) -> Result<bool, TypingError> {
    assert!(
        subst.get(tvar).is_none(),
        "ICE join_bind_tvar called on bound tvar"
    );

    fn used_tvars(used: &mut BTreeMap<TVar, Loc>, sp!(loc, t_): &Type) {
        use Type_ as T;
        match t_ {
            T::Var(v) => {
                used.insert(*v, *loc);
            }
            T::Ref(_, inner) => used_tvars(used, inner),
            T::AutoRef(_, inner) => used_tvars(used, inner),
            T::Apply(_, _, inners) => inners
                .iter()
                .rev()
                .for_each(|inner| used_tvars(used, inner)),
            T::Fun(inner_args, inner_ret) => {
                inner_args
                    .iter()
                    .rev()
                    .for_each(|inner| used_tvars(used, inner));
                used_tvars(used, inner_ret)
            }
            T::Unit | T::Param(_) | T::Anything | T::UnresolvedError => (),
        }
    }

    // check not necessary for soundness but improves error message structure
    if !check_num_tvar(subst, loc, tvar, &ty) {
        return Ok(false);
    }

    let used = &mut BTreeMap::new();
    used_tvars(used, &ty);
    if let Some(_rec_loc) = used.get(&tvar) {
        return Err(TypingError::RecursiveType(loc));
    }

    match &ty.value {
        Type_::Anything => (),
        _ => subst.insert(tvar, ty),
    }
    Ok(true)
}

fn check_num_tvar(subst: &Subst, _loc: Loc, tvar: TVar, ty: &Type) -> bool {
    !subst.is_num_var(tvar) || check_num_tvar_(subst, ty)
}

fn check_num_tvar_(subst: &Subst, ty: &Type) -> bool {
    use Type_::*;
    match &ty.value {
        UnresolvedError | Anything => true,
        Apply(_, sp!(_, TypeName_::Builtin(sp!(_, bt))), _) => bt.is_numeric(),

        Var(v) => {
            let last_tvar = forward_tvar(subst, *v);
            match subst.get(last_tvar) {
                Some(sp!(_, Var(_))) => unreachable!(),
                None => subst.is_num_var(last_tvar),
                Some(t) => check_num_tvar_(subst, t),
            }
        }
        _ => false,
    }
}

//--------------------------------------------------------------------------------------------------
// Join for Ref Vars
//--------------------------------------------------------------------------------------------------

fn join_autoref(
    mut subst: Subst,
    case: TypingCase,
    loc1: Loc,
    id1: RefVar,
    lhs: &Type,
    loc2: Loc,
    id2: RefVar,
    rhs: &Type,
) -> Result<(Subst, Type), TypingError> {
    use Type_::AutoRef;
    let ref_var1 = forward_ref_var(&subst, id1);
    let ref_var2 = forward_ref_var(&subst, id2);

    let ref_kind1 = subst.get_ref_var(ref_var1);
    let ref_kind2 = subst.get_ref_var(ref_var2);

    let new_ref_var = RefVar::next();

    match (ref_kind1, ref_kind2) {
        (None, None) => {
            // If both auto-refs are unset, combine them and `join_impl` their types.
            subst.insert_ref_var(ref_var1, RefKind::Forward(new_ref_var));
            subst.insert_ref_var(ref_var2, RefKind::Forward(new_ref_var));
            let (subst, ty) = join_impl(subst, case, lhs, rhs)?;
            Ok((subst, sp(loc2, AutoRef(new_ref_var, Box::new(ty)))))
        }
        (Some(rk1), Some(rk2)) => {
            assert!(!matches!(rk1, RefKind::Forward(_)));
            assert!(!matches!(rk2, RefKind::Forward(_)));
            match &case {
                TypingCase::Join => {
                    if let Some(join_kind) = rk1.join(rk2) {
                        // If both auto-refs are set and can join, we join them.
                        subst.insert_ref_var(ref_var1, RefKind::Forward(new_ref_var));
                        subst.insert_ref_var(ref_var2, RefKind::Forward(new_ref_var));
                        subst.insert_ref_var(new_ref_var, join_kind);
                        let (subst, ty) = join_impl(subst, case, lhs, rhs)?;
                        Ok((subst, sp(loc2, AutoRef(new_ref_var, Box::new(ty)))))
                    } else {
                        // If both already set without a join, realize and recur for an error.
                        let lhs = realize_autoref(loc1, rk1, lhs.clone());
                        let rhs = realize_autoref(loc2, rk2, rhs.clone());
                        join_impl(subst, case, &lhs, &rhs)
                    }
                }
                TypingCase::Subtype => {
                    // We can realize both and ask `join_impl` if this is a valid subtype.
                    let lhs = realize_autoref(loc1, rk1, lhs.clone());
                    let rhs = realize_autoref(loc2, rk2, rhs.clone());
                    join_impl(subst, case, &lhs, &rhs)
                }
            }
        }
        (None, Some(rk2)) => {
            let lhs = sp(loc1, AutoRef(ref_var1, Box::new(lhs.clone())));
            let rhs = realize_autoref(loc2, rk2, rhs.clone());
            join_impl(subst, case, &lhs, &rhs)
        }
        (Some(rk1), None) => {
            let lhs = realize_autoref(loc1, rk1, lhs.clone());
            let rhs = sp(loc2, AutoRef(ref_var2, Box::new(rhs.clone())));
            join_impl(subst, case, &lhs, &rhs)
        }
    }
}

fn join_bind_autoref(
    mut subst: Subst,
    case: TypingCase,
    loc: Loc,
    rv: RefVar,
    rvtype: &Type,
    other @ sp!(_, otype): &Type,
) -> Result<(Subst, Type), TypingError> {
    use Type_::*;
    let rv = forward_ref_var(&subst, rv);
    match subst.get_ref_var(rv) {
        Some(RefKind::Forward(_)) => unreachable!(),
        None => match otype {
            Ref(false, t) => {
                subst.insert_ref_var(rv, RefKind::ImmRef);
                join_impl(subst, case, rvtype, t)
            }
            Ref(true, t) => {
                subst.insert_ref_var(rv, RefKind::MutRef);
                join_impl(subst, case, rvtype, t)
            }
            Anything | UnresolvedError => Ok((subst, other.clone())),
            Unit | Param(_) | Apply(_, _, _) | Fun(_, _) => {
                subst.insert_ref_var(rv, RefKind::Value);
                join_impl(subst, case, rvtype, other)
            }
            // Both of these cases should have been handled in `join_impl` instead of flowing here.
            AutoRef(_, _) | Var(_) => unreachable!(),
        },
        Some(ref_kind) => {
            let rvtype = realize_autoref(loc, ref_kind, rvtype.clone());
            join_impl(subst, case, &rvtype, other)
        }
    }
}

fn realize_autoref(loc: Loc, ref_kind: &RefKind, ty: Type) -> Type {
    use Type_::Ref;
    match ref_kind {
        RefKind::Value => ty,
        RefKind::ImmRef => sp(loc, Ref(false, Box::new(ty))),
        RefKind::MutRef => sp(loc, Ref(true, Box::new(ty))),
        RefKind::Forward(_) => panic!("ICE must unfold ref var before realizing"),
    }
}

fn forward_ref_var(subst: &Subst, id: RefVar) -> RefVar {
    let mut cur = id;
    loop {
        match subst.get_ref_var(cur) {
            Some(RefKind::Forward(next)) => cur = *next,
            Some(_) | None => break cur,
        }
    }
}
