//# init --edition 2024.alpha

//# publish
module 0x42::m {

    public struct S has copy, drop { t: u64 }

    public fun make_s(t: u64): S {
        S { t }
    }

    public fun t00(a: S, b: S): bool {
       a == b
    }

    public fun t01(a: &S, b: S): bool {
       a == b
    }

    public fun t02(a: S, b: &S): bool {
       a == b
    }

    public fun t03(a: &mut S, mut b: S): bool {
       a == b
    }

    public fun t04(mut a: S, b: &mut S): bool {
       a == b
    }

    public fun t05(a: &S, b: &S): bool {
       a == b
    }

    public fun t06(a: &mut S, b: &S): bool {
       a == b
    }

    public fun t07(a: &S, b: &mut S): bool {
       a == b
    }

    public fun t08(a: &mut S, b: &mut S): bool {
       a == b
    }

    public fun t09<S: drop>(a: S, b: S): bool {
       a == b
    }

    public fun t10<S: drop>(a: &S, b: S): bool {
       a == b
    }

    public fun t11<S: drop>(a: S, b: &S): bool {
       a == b
    }

    public fun t12<S: drop>(a: &mut S, mut b: S): bool {
       a == b
    }

    public fun t13<S: drop>(mut a: S, b: &mut S): bool {
       a == b
    }

    public fun t14<S: drop>(a: &S, b: &S): bool {
       a == b
    }

    public fun t15<S: drop>(a: &mut S, b: &S): bool {
       a == b
    }

    public fun t16<S: drop>(a: &S, b: &mut S): bool {
       a == b
    }

    public fun t17<S: drop>(a: &mut S, b: &mut S): bool {
       a == b
    }

    public fun tnum_0(): bool {
       0 == &0
    }

    public fun tnum_1(): bool {
       &0 == &0
    }

    public fun tnum_2(): bool {
        let mut a = 0;
        let b = &mut 0;
        let c = &0;
        a == b && b == c && a == c
    }


}

//# run
module 0x42::main {

    fun main() {
        let s_val = 0x42::m::make_s(42);
        let s_ref = &(0x42::m::make_s(42));
        let s_mut = &mut (0x42::m::make_s(42));

        assert!(0x42::m::t00(s_val, s_val), 0);
        assert!(0x42::m::t01(s_ref, s_val), 0);
        assert!(0x42::m::t02(s_val, s_ref), 0);
        assert!(0x42::m::t03(s_mut, s_val), 0);
        assert!(0x42::m::t04(s_val, s_mut), 0);
        assert!(0x42::m::t05(s_ref, s_ref), 0);
        assert!(0x42::m::t06(s_mut, s_ref), 0);
        assert!(0x42::m::t07(s_ref, s_mut), 0);
        // can't double-borrow the mut here
        // assert!(0x42::m::t08(s_mut, s_mut), 0);
        assert!(0x42::m::t09(s_val, s_val), 0);
        assert!(0x42::m::t10(s_ref, s_val), 0);
        assert!(0x42::m::t11(s_val, s_ref), 0);
        assert!(0x42::m::t12(s_mut, s_val), 0);
        assert!(0x42::m::t13(s_val, s_mut), 0);
        assert!(0x42::m::t14(s_ref, s_ref), 0);
        assert!(0x42::m::t15(s_mut, s_ref), 0);
        assert!(0x42::m::t16(s_ref, s_mut), 0);
        // can't double-borrow the mut here
        // assert!(0x42::m::t17(s_mut, s_mut), 0);

        let s2_val = 0x42::m::make_s(2);
        let s2_ref = &(0x42::m::make_s(2));
        let s2_mut = &mut (0x42::m::make_s(2));

        assert!(!0x42::m::t00(s_val, s2_val), 0);
        assert!(!0x42::m::t01(s_ref, s2_val), 0);
        assert!(!0x42::m::t02(s_val, s2_ref), 0);
        assert!(!0x42::m::t03(s_mut, s2_val), 0);
        assert!(!0x42::m::t04(s_val, s2_mut), 0);
        assert!(!0x42::m::t05(s_ref, s2_ref), 0);
        assert!(!0x42::m::t06(s_mut, s2_ref), 0);
        assert!(!0x42::m::t07(s_ref, s2_mut), 0);
        assert!(!0x42::m::t08(s_mut, s2_mut), 0);
        assert!(!0x42::m::t09(s_val, s2_val), 0);
        assert!(!0x42::m::t10(s_ref, s2_val), 0);
        assert!(!0x42::m::t11(s_val, s2_ref), 0);
        assert!(!0x42::m::t12(s_mut, s2_val), 0);
        assert!(!0x42::m::t13(s_val, s2_mut), 0);
        assert!(!0x42::m::t14(s_ref, s2_ref), 0);
        assert!(!0x42::m::t15(s_mut, s2_ref), 0);
        assert!(!0x42::m::t16(s_ref, s2_mut), 0);
        assert!(!0x42::m::t17(s_mut, s2_mut), 0);

        assert!(0x42::m::tnum_0(), 0);
        assert!(0x42::m::tnum_1(), 0);
        assert!(0x42::m::tnum_2(), 0);
    }
}
