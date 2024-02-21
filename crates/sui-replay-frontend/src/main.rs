// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use sui_replay_frontend::generate_html_from_json;

const DATA: &str = "{\"effects\":{\"messageVersion\":\"v1\",\"status\":{\"status\":\"success\"},\"executedEpoch\":\"309\",\"gasUsed\":{\"computationCost\":\"6750000\",\"storageCost\":\"10769200\",\"storageRebate\":\"10661508\",\"nonRefundableStorageFee\":\"107692\"},\"modifiedAtVersions\":[{\"objectId\":\"0x00dfb2bc2b2744ed18774377cc9688ca907ff6390015801a9a7917e7388a0cd2\",\"sequenceNumber\":\"69073435\"},{\"objectId\":\"0x178355b2161faf98a017f9c3abecaf1ac837b231b669c3df70baf84b96392caf\",\"sequenceNumber\":\"69073423\"},{\"objectId\":\"0x29e37978cb1c9501bda5d7c105f24f0058bc1668637e307fbc290dba48cb918d\",\"sequenceNumber\":\"69073435\"},{\"objectId\":\"0x6e31a26e9f903db7b18919d4bdd7b7634ee890bc631bf116e6f9f24ab16f1315\",\"sequenceNumber\":\"69073435\"},{\"objectId\":\"0xb17a4748ce1e9e153886291e5729a3ae53f6596d33a5a6af4ef36d7f0c04e70f\",\"sequenceNumber\":\"69073435\"},{\"objectId\":\"0xd6bc9908164382e1ca949ee1809cfe0a52bceed2cff35c0493ff5221cf55849e\",\"sequenceNumber\":\"69073435\"},{\"objectId\":\"0xf28e4e40a96948a19805fe76a4da373a262daac47588bac78d73033bce7b5a38\",\"sequenceNumber\":\"69073435\"}],\"sharedObjects\":[{\"objectId\":\"0x29e37978cb1c9501bda5d7c105f24f0058bc1668637e307fbc290dba48cb918d\",\"version\":69073435,\"digest\":\"6amtH2knbFdmZZNq4DD7LRCKTrWXsfPKMRFSGrSusbyn\"},{\"objectId\":\"0x6e31a26e9f903db7b18919d4bdd7b7634ee890bc631bf116e6f9f24ab16f1315\",\"version\":69073435,\"digest\":\"E5uc8ZDK9hAjAmZeWne3AfXw6FCTRLDGtzKHAzFAzMaz\"},{\"objectId\":\"0xf28e4e40a96948a19805fe76a4da373a262daac47588bac78d73033bce7b5a38\",\"version\":69073435,\"digest\":\"EyWJzAcTp2ve9K9EMyJ8CLyQrQBrMnRQiL1f3UQLSCXE\"},{\"objectId\":\"0xaeab97f96cf9877fee2883315d459552b2b921edc16d7ceac6eab944dd88919c\",\"version\":69073359,\"digest\":\"9cdoahz64sbi8DrtuiqZtMRLSDXWdrdGUkdgvjhtbCzZ\"},{\"objectId\":\"0x0000000000000000000000000000000000000000000000000000000000000006\",\"version\":26386860,\"digest\":\"3Jm7AyNF6Me1cGeZnvwkyfNAnEiSW9VK8Sr96ydR4cxQ\"},{\"objectId\":\"0x1f9310238ee9298fb703c3419030b35b22bb1cc37113e3bb5007c99aec79e5b8\",\"version\":69072943,\"digest\":\"4SrYNGzXGYMCJgBCv5JmKBKM1EEueeUQzs1Uwne3KaQK\"}],\"transactionDigest\":\"4ZW9TDgYcEcJCmn1hfnqCkZGSijig2Q7LGjHog8sXFno\",\"mutated\":[{\"owner\":{\"ObjectOwner\":\"0x402138636c05227b26ecf7e8922dd654bbf9eaf106a2ecb0d12976a4f49a025c\"},\"reference\":{\"objectId\":\"0x00dfb2bc2b2744ed18774377cc9688ca907ff6390015801a9a7917e7388a0cd2\",\"version\":69073436,\"digest\":\"7c17c82QxtNdNoWLpGXMwkp5oQYWAeXNQWAawGfNKsc7\"}},{\"owner\":{\"AddressOwner\":\"0x02a212de6a9dfa3a69e22387acfbafbb1a9e591bd9d636e7895dcfc8de05f331\"},\"reference\":{\"objectId\":\"0x178355b2161faf98a017f9c3abecaf1ac837b231b669c3df70baf84b96392caf\",\"version\":69073436,\"digest\":\"EqiXRXqYiF6ZG3nQCTA2g5BvuQbwiZWzV4XA4mGLdiAn\"}},{\"owner\":{\"Shared\":{\"initial_shared_version\":20120875}},\"reference\":{\"objectId\":\"0x29e37978cb1c9501bda5d7c105f24f0058bc1668637e307fbc290dba48cb918d\",\"version\":69073436,\"digest\":\"5B94i6Yvc1YKYMiCF5XB4NMmr4jJTn3ex6n1i4gUVyBh\"}},{\"owner\":{\"Shared\":{\"initial_shared_version\":20120865}},\"reference\":{\"objectId\":\"0x6e31a26e9f903db7b18919d4bdd7b7634ee890bc631bf116e6f9f24ab16f1315\",\"version\":69073436,\"digest\":\"AxNpsFESQWANXkbUsQkBXRz23RDyaxBkq5YNvWjqSeMo\"}},{\"owner\":{\"ObjectOwner\":\"0x10f4c2ef7e9a634453fa21722c0177a6c7920d5bc5ea8eb1d5c89a82b66c1d85\"},\"reference\":{\"objectId\":\"0xb17a4748ce1e9e153886291e5729a3ae53f6596d33a5a6af4ef36d7f0c04e70f\",\"version\":69073436,\"digest\":\"9kwLWn5pXoH15FjMTZJQi3f5F8UUM1LxmVJUc4f1TxFX\"}},{\"owner\":{\"ObjectOwner\":\"0x79db959a65bdd419caa2dd7d40e9f2bd6342cb56be78018e297db37681fcd44c\"},\"reference\":{\"objectId\":\"0xd6bc9908164382e1ca949ee1809cfe0a52bceed2cff35c0493ff5221cf55849e\",\"version\":69073436,\"digest\":\"Gk8gmfotbuY14VuKojDn3fKAxD1zoQ7AbkpnSUDFUQDH\"}},{\"owner\":{\"Shared\":{\"initial_shared_version\":20119534}},\"reference\":{\"objectId\":\"0xf28e4e40a96948a19805fe76a4da373a262daac47588bac78d73033bce7b5a38\",\"version\":69073436,\"digest\":\"FkKaMrv68rnLQtFFMDvhaZ5qjdKHAyMY1DkUPzCWqdG\"}}],\"gasObject\":{\"owner\":{\"AddressOwner\":\"0x02a212de6a9dfa3a69e22387acfbafbb1a9e591bd9d636e7895dcfc8de05f331\"},\"reference\":{\"objectId\":\"0x178355b2161faf98a017f9c3abecaf1ac837b231b669c3df70baf84b96392caf\",\"version\":69073436,\"digest\":\"EqiXRXqYiF6ZG3nQCTA2g5BvuQbwiZWzV4XA4mGLdiAn\"}},\"eventsDigest\":\"6ayfbCQ6Qv4UMUyLPQkhiZh979jKgAdJnfoSWDpY7kkR\",\"dependencies\":[\"4P9i7JUSRoyTY41G5AbUkn7Rf1nurWtVkekz6pGSe7SX\",\"6gu7AhtWe1m8zC9DoTc9VkoA9z71uQ9L6h9wFPQSK2cY\",\"6kqPcXYYddXGruh5pVc2s9ruQYFadDkFyKD8zwxXf5cH\",\"9aXs2UJPyeQHV8gq15yXHU91LDjaetPvNRZAVonX2TUh\",\"C3wFcACFqgmaDqW3BArZBbb8gKc6MSrjHUYft6d7FWa3\",\"EA15NafdqJajL4AStDbyxxx7sLUvGRk68oYV9vv8pA8q\",\"FBrRPd2HQ75SRj1tuN5R68NDtXhCrmK3JAVpJfDjfPDD\"]},\"gas_status\":{\"V2\":{\"gas_status\":{\"gas_model_version\":8,\"cost_table\":{\"instruction_tiers\":{\"0\":1,\"20000\":2,\"50000\":10,\"100000\":50,\"200000\":100,\"10000000\":1000},\"stack_height_tiers\":{\"0\":1,\"1000\":2,\"10000\":10},\"stack_size_tiers\":{\"0\":1,\"100000\":2,\"500000\":5,\"1000000\":100,\"100000000\":1000}},\"gas_left\":{\"val\":657769084,\"phantom\":null},\"gas_price\":750,\"initial_budget\":{\"val\":666666000,\"phantom\":null},\"charge\":true,\"stack_height_high_water_mark\":9125,\"stack_height_current\":9049,\"stack_height_next_tier_start\":10000,\"stack_height_current_tier_mult\":2,\"stack_size_high_water_mark\":992196,\"stack_size_current\":992196,\"stack_size_next_tier_start\":1000000,\"stack_size_current_tier_mult\":5,\"instructions_executed\":176309,\"instructions_next_tier_start\":200000,\"instructions_current_tier_mult\":50,\"profiler\":null},\"cost_table\":{\"min_transaction_cost\":750000,\"max_gas_budget\":50000000000,\"package_publish_per_byte_cost\":80,\"object_read_per_byte_cost\":15,\"storage_per_byte_cost\":100,\"execution_cost_table\":{\"instruction_tiers\":{\"0\":1,\"20000\":2,\"50000\":10,\"100000\":50,\"200000\":100,\"10000000\":1000},\"stack_height_tiers\":{\"0\":1,\"1000\":2,\"10000\":10},\"stack_size_tiers\":{\"0\":1,\"100000\":2,\"500000\":5,\"1000000\":100,\"100000000\":1000}},\"computation_bucket\":[{\"min\":0,\"max\":1000,\"cost\":1000},{\"min\":1000,\"max\":5000,\"cost\":5000},{\"min\":5000,\"max\":10000,\"cost\":10000},{\"min\":10000,\"max\":20000,\"cost\":20000},{\"min\":20000,\"max\":50000,\"cost\":50000},{\"min\":50000,\"max\":200000,\"cost\":200000},{\"min\":200000,\"max\":1000000,\"cost\":1000000},{\"min\":1000000,\"max\":5000000,\"cost\":5000000}]},\"gas_budget\":500000000,\"computation_cost\":6750000,\"charge\":true,\"gas_price\":750,\"reference_gas_price\":750,\"storage_gas_price\":76,\"per_object_storage\":[[\"0x00dfb2bc2b2744ed18774377cc9688ca907ff6390015801a9a7917e7388a0cd2\",{\"storage_cost\":988000,\"storage_rebate\":988000,\"new_size\":130}],[\"0x178355b2161faf98a017f9c3abecaf1ac837b231b669c3df70baf84b96392caf\",{\"storage_cost\":988000,\"storage_rebate\":988000,\"new_size\":130}],[\"0x29e37978cb1c9501bda5d7c105f24f0058bc1668637e307fbc290dba48cb918d\",{\"storage_cost\":2272400,\"storage_rebate\":2272400,\"new_size\":299}],[\"0x6e31a26e9f903db7b18919d4bdd7b7634ee890bc631bf116e6f9f24ab16f1315\",{\"storage_cost\":2272400,\"storage_rebate\":2272400,\"new_size\":299}],[\"0xb17a4748ce1e9e153886291e5729a3ae53f6596d33a5a6af4ef36d7f0c04e70f\",{\"storage_cost\":988000,\"storage_rebate\":988000,\"new_size\":130}],[\"0xd6bc9908164382e1ca949ee1809cfe0a52bceed2cff35c0493ff5221cf55849e\",{\"storage_cost\":988000,\"storage_rebate\":988000,\"new_size\":130}],[\"0xf28e4e40a96948a19805fe76a4da373a262daac47588bac78d73033bce7b5a38\",{\"storage_cost\":2272400,\"storage_rebate\":2272400,\"new_size\":299}]],\"rebate_rate\":9900,\"unmetered_storage_rebate\":0,\"gas_rounding_step\":1000}},\"transaction_info\":{\"ProgrammableTransaction\":{\"inputs\":[{\"Object\":{\"SharedObject\":{\"id\":\"0xaeab97f96cf9877fee2883315d459552b2b921edc16d7ceac6eab944dd88919c\",\"initial_shared_version\":64,\"mutable\":false}}},{\"Pure\":[184,7,1,0,0,0,3,13,2,159,179,58,54,163,133,6,146,196,244,216,147,201,152,245,67,147,165,220,191,191,179,221,136,19,4,152,201,140,253,223,69,71,143,159,107,8,179,41,25,28,96,139,173,223,21,133,134,175,154,98,137,33,4,205,127,220,43,202,135,247,8,131,244,1,3,220,92,168,169,215,36,34,87,61,169,117,215,153,49,86,128,14,119,46,39,111,12,4,169,150,177,10,61,72,35,105,179,104,66,170,11,128,42,191,102,125,30,91,76,34,55,79,175,255,232,196,18,126,227,223,254,180,63,175,39,25,63,125,77,0,4,97,157,200,199,154,243,66,103,19,45,214,76,198,174,64,195,117,144,4,183,92,243,208,4,207,34,10,235,240,44,182,71,20,186,160,125,193,196,158,30,12,111,55,36,232,183,131,3,92,116,98,18,66,41,193,131,242,189,234,19,84,113,126,7,0,6,87,8,235,7,69,159,23,242,170,218,188,160,181,74,86,3,174,106,31,215,188,96,233,71,158,83,198,230,46,24,19,93,49,37,8,21,2,87,180,102,199,141,126,7,210,72,30,89,31,213,183,18,185,240,137,55,243,161,67,180,223,46,208,218,0,7,168,142,153,79,90,60,192,68,215,91,154,14,153,52,189,223,34,168,114,67,138,22,29,75,46,13,188,208,106,136,98,228,4,131,79,115,190,160,15,198,202,178,27,80,172,181,215,194,27,144,102,234,180,198,255,163,180,76,0,186,206,75,126,181,0,8,190,75,139,135,4,108,249,222,204,243,205,34,132,113,216,203,102,245,32,20,126,51,246,184,225,232,162,254,120,109,24,251,76,98,177,25,114,95,1,220,240,74,49,8,192,178,147,166,17,148,185,105,101,5,160,14,133,1,144,127,9,151,13,74,0,10,128,91,120,75,50,123,161,234,121,94,97,166,143,52,216,100,152,161,146,163,239,118,95,200,56,205,192,150,243,132,230,134,111,218,80,5,87,252,249,179,61,36,56,249,146,137,169,105,73,156,5,78,29,140,43,145,61,38,35,196,91,224,160,201,1,11,77,223,198,156,90,175,86,106,138,250,234,82,230,244,83,167,37,73,174,5,17,17,205,114,236,18,171,59,159,132,230,139,99,30,32,49,22,171,213,145,7,92,111,141,109,3,7,254,211,74,201,30,67,51,206,0,124,30,80,39,52,193,210,3,0,12,219,12,113,248,45,190,34,19,186,245,216,85,44,74,92,26,38,32,125,253,59,194,43,36,79,214,34,136,129,236,70,239,40,180,75,165,216,13,243,23,32,246,56,59,66,31,177,209,92,200,249,86,41,34,4,235,10,127,175,30,232,105,121,146,1,13,222,49,135,63,16,162,175,134,64,116,210,76,187,138,124,2,29,47,99,167,127,39,108,240,252,43,36,203,149,111,66,140,9,148,114,250,190,162,83,118,88,207,183,59,227,115,220,175,66,67,199,219,140,5,107,136,228,112,242,117,122,230,232,54,0,14,65,174,25,91,25,129,128,199,199,199,161,120,211,20,210,63,147,138,0,190,51,140,49,142,139,138,168,210,144,159,99,242,6,69,188,78,152,253,147,239,248,225,134,104,21,187,138,246,232,164,32,92,11,133,103,125,102,78,252,112,99,135,225,36,1,16,81,133,60,66,246,247,194,139,86,147,181,15,182,236,232,114,215,26,254,226,193,191,254,74,204,232,31,79,197,191,105,3,60,228,52,76,178,247,6,102,174,219,228,73,149,92,165,227,70,223,172,244,208,39,223,149,39,192,106,209,6,25,151,15,1,18,1,58,208,180,79,153,56,144,180,246,30,196,91,240,129,47,5,225,134,82,129,161,176,21,61,40,163,84,184,140,178,147,77,74,142,55,198,212,96,163,179,106,28,27,88,91,252,80,163,254,136,14,93,142,245,230,128,214,98,67,108,126,243,117,0,101,207,37,150,0,0,0,0,0,26,225,1,250,237,172,88,81,227,43,155,35,181,249,65,26,140,43,172,74,174,62,212,221,123,129,29,209,167,46,164,170,113,0,0,0,0,2,114,149,69,1,65,85,87,86,0,0,0,0,0,7,122,146,245,0,0,39,16,173,242,234,87,170,174,152,149,59,69,246,85,233,204,252,180,103,105,110,66]},{\"Object\":{\"SharedObject\":{\"id\":\"0x0000000000000000000000000000000000000000000000000000000000000006\",\"initial_shared_version\":1,\"mutable\":false}}},{\"Object\":{\"SharedObject\":{\"id\":\"0x1f9310238ee9298fb703c3419030b35b22bb1cc37113e3bb5007c99aec79e5b8\",\"initial_shared_version\":17589382,\"mutable\":false}}},{\"Pure\":[163,14,80,78,65,85,1,0,0,0,3,184,1,0,0,0,3,13,2,159,179,58,54,163,133,6,146,196,244,216,147,201,152,245,67,147,165,220,191,191,179,221,136,19,4,152,201,140,253,223,69,71,143,159,107,8,179,41,25,28,96,139,173,223,21,133,134,175,154,98,137,33,4,205,127,220,43,202,135,247,8,131,244,1,3,220,92,168,169,215,36,34,87,61,169,117,215,153,49,86,128,14,119,46,39,111,12,4,169,150,177,10,61,72,35,105,179,104,66,170,11,128,42,191,102,125,30,91,76,34,55,79,175,255,232,196,18,126,227,223,254,180,63,175,39,25,63,125,77,0,4,97,157,200,199,154,243,66,103,19,45,214,76,198,174,64,195,117,144,4,183,92,243,208,4,207,34,10,235,240,44,182,71,20,186,160,125,193,196,158,30,12,111,55,36,232,183,131,3,92,116,98,18,66,41,193,131,242,189,234,19,84,113,126,7,0,6,87,8,235,7,69,159,23,242,170,218,188,160,181,74,86,3,174,106,31,215,188,96,233,71,158,83,198,230,46,24,19,93,49,37,8,21,2,87,180,102,199,141,126,7,210,72,30,89,31,213,183,18,185,240,137,55,243,161,67,180,223,46,208,218,0,7,168,142,153,79,90,60,192,68,215,91,154,14,153,52,189,223,34,168,114,67,138,22,29,75,46,13,188,208,106,136,98,228,4,131,79,115,190,160,15,198,202,178,27,80,172,181,215,194,27,144,102,234,180,198,255,163,180,76,0,186,206,75,126,181,0,8,190,75,139,135,4,108,249,222,204,243,205,34,132,113,216,203,102,245,32,20,126,51,246,184,225,232,162,254,120,109,24,251,76,98,177,25,114,95,1,220,240,74,49,8,192,178,147,166,17,148,185,105,101,5,160,14,133,1,144,127,9,151,13,74,0,10,128,91,120,75,50,123,161,234,121,94,97,166,143,52,216,100,152,161,146,163,239,118,95,200,56,205,192,150,243,132,230,134,111,218,80,5,87,252,249,179,61,36,56,249,146,137,169,105,73,156,5,78,29,140,43,145,61,38,35,196,91,224,160,201,1,11,77,223,198,156,90,175,86,106,138,250,234,82,230,244,83,167,37,73,174,5,17,17,205,114,236,18,171,59,159,132,230,139,99,30,32,49,22,171,213,145,7,92,111,141,109,3,7,254,211,74,201,30,67,51,206,0,124,30,80,39,52,193,210,3,0,12,219,12,113,248,45,190,34,19,186,245,216,85,44,74,92,26,38,32,125,253,59,194,43,36,79,214,34,136,129,236,70,239,40,180,75,165,216,13,243,23,32,246,56,59,66,31,177,209,92,200,249,86,41,34,4,235,10,127,175,30,232,105,121,146,1,13,222,49,135,63,16,162,175,134,64,116,210,76,187,138,124,2,29,47,99,167,127,39,108,240,252,43,36,203,149,111,66,140,9,148,114,250,190,162,83,118,88,207,183,59,227,115,220,175,66,67,199,219,140,5,107,136,228,112,242,117,122,230,232,54,0,14,65,174,25,91,25,129,128,199,199,199,161,120,211,20,210,63,147,138,0,190,51,140,49,142,139,138,168,210,144,159,99,242,6,69,188,78,152,253,147,239,248,225,134,104,21,187,138,246,232,164,32,92,11,133,103,125,102,78,252,112,99,135,225,36,1,16,81,133,60,66,246,247,194,139,86,147,181,15,182,236,232,114,215,26,254,226,193,191,254,74,204,232,31,79,197,191,105,3,60,228,52,76,178,247,6,102,174,219,228,73,149,92,165,227,70,223,172,244,208,39,223,149,39,192,106,209,6,25,151,15,1,18,1,58,208,180,79,153,56,144,180,246,30,196,91,240,129,47,5,225,134,82,129,161,176,21,61,40,163,84,184,140,178,147,77,74,142,55,198,212,96,163,179,106,28,27,88,91,252,80,163,254,136,14,93,142,245,230,128,214,98,67,108,126,243,117,0,101,207,37,150,0,0,0,0,0,26,225,1,250,237,172,88,81,227,43,155,35,181,249,65,26,140,43,172,74,174,62,212,221,123,129,29,209,167,46,164,170,113,0,0,0,0,2,114,149,69,1,65,85,87,86,0,0,0,0,0,7,122,146,245,0,0,39,16,173,242,234,87,170,174,152,149,59,69,246,85,233,204,252,180,103,105,110,66,3,0,85,0,42,1,222,174,201,229,26,87,146,119,179,75,18,35,153,152,77,11,191,87,226,69,138,126,66,254,205,40,41,134,122,13,0,0,0,0,3,142,41,150,0,0,0,0,0,0,188,108,255,255,255,248,0,0,0,0,101,207,37,150,0,0,0,0,101,207,37,150,0,0,0,0,3,147,3,10,0,0,0,0,0,0,177,20,10,201,255,242,44,97,235,47,139,82,117,173,120,186,124,235,0,252,65,206,60,205,222,229,51,142,9,108,135,233,123,105,223,119,220,97,70,220,147,220,240,159,54,31,28,253,2,10,250,247,95,203,15,123,205,230,101,207,249,88,89,139,160,253,116,160,68,154,235,52,101,205,181,39,242,10,141,211,32,195,130,12,253,167,37,221,25,44,130,235,113,80,226,95,177,242,142,39,164,223,95,97,71,145,63,121,248,109,141,216,79,147,120,222,210,163,174,105,130,92,245,209,100,92,218,193,97,94,153,33,82,248,80,252,83,82,238,188,55,36,11,73,72,50,136,136,238,22,150,242,69,16,246,164,189,12,206,146,145,107,62,232,150,228,234,116,153,62,98,82,72,178,180,180,8,42,132,217,13,117,24,189,144,220,199,220,64,211,15,191,123,63,3,229,245,182,49,77,143,148,28,0,85,0,63,164,37,40,72,249,240,161,72,11,230,39,69,164,98,157,158,177,50,42,235,171,138,121,30,52,75,59,156,26,220,245,0,0,0,0,12,20,229,99,0,0,0,0,0,3,71,154,255,255,255,248,0,0,0,0,101,207,37,150,0,0,0,0,101,207,37,150,0,0,0,0,12,48,113,120,0,0,0,0,0,3,106,24,10,0,52,93,75,185,120,248,61,140,50,33,6,192,115,217,38,121,157,19,223,235,180,203,80,82,21,179,204,211,183,184,73,146,87,43,64,24,73,19,163,60,11,122,50,97,234,201,146,173,170,235,230,82,20,205,128,72,71,105,13,215,0,105,53,116,152,59,238,163,34,158,212,230,228,129,180,129,251,140,248,117,22,133,65,5,65,139,151,135,144,45,204,69,14,111,40,254,86,159,43,100,112,188,16,111,78,87,187,48,128,102,11,69,112,174,128,8,220,90,199,209,100,92,218,193,97,94,153,33,82,248,80,252,83,82,238,188,55,36,11,73,72,50,136,136,238,22,150,242,69,16,246,164,189,12,206,146,145,107,62,232,150,228,234,116,153,62,98,82,72,178,180,180,8,42,132,217,13,117,24,189,144,220,199,220,64,211,15,191,123,63,3,229,245,182,49,77,143,148,28,0,85,0,147,218,51,82,249,241,209,5,253,254,73,113,207,168,14,157,215,119,191,197,208,246,131,235,182,225,41,75,146,19,123,183,0,0,0,0,242,61,62,160,0,0,0,0,0,43,238,24,255,255,255,248,0,0,0,0,101,207,37,150,0,0,0,0,101,207,37,150,0,0,0,0,244,59,128,104,0,0,0,0,0,48,51,22,10,242,59,60,17,14,82,60,87,70,124,126,33,132,194,23,183,83,55,195,140,25,202,157,109,144,5,63,182,71,243,11,57,100,193,66,213,9,45,198,135,227,124,85,216,226,87,245,52,66,66,160,60,163,167,143,157,217,17,42,226,13,38,190,255,55,115,142,219,33,199,188,101,182,179,95,122,39,236,15,90,110,126,97,52,180,111,35,13,86,207,88,97,195,40,106,114,79,207,50,228,214,224,11,154,197,114,15,210,17,77,159,149,220,142,77,228,112,104,24,188,241,240,94,107,226,156,15,159,187,111,81,45,182,249,236,216,164,174,137,201,102,134,46,208,235,30,69,125,216,108,72,244,103,71,45,33,223,153,130,157,2,14,236,253,218,202,232,195,159,129,225,29,186,69,196,20,100,64,104,118,189,144,220,199,220,64,211,15,191,123,63,3,229,245,182,49,77,143,148,28]},{\"Pure\":[1,0,0,0,0,0,0,0]},{\"Pure\":[1,0,0,0,0,0,0,0]},{\"Pure\":[1,0,0,0,0,0,0,0]},{\"Object\":{\"SharedObject\":{\"id\":\"0xf28e4e40a96948a19805fe76a4da373a262daac47588bac78d73033bce7b5a38\",\"initial_shared_version\":20119534,\"mutable\":true}}},{\"Object\":{\"SharedObject\":{\"id\":\"0x6e31a26e9f903db7b18919d4bdd7b7634ee890bc631bf116e6f9f24ab16f1315\",\"initial_shared_version\":20120865,\"mutable\":true}}},{\"Object\":{\"SharedObject\":{\"id\":\"0x29e37978cb1c9501bda5d7c105f24f0058bc1668637e307fbc290dba48cb918d\",\"initial_shared_version\":20120875,\"mutable\":true}}}],\"commands\":[{\"MoveCall\":{\"package\":\"0x5306f64e312b581766351c07af79c72fcb1cd25147157fdc2f8ad76de9a3fb6a\",\"module\":\"vaa\",\"function\":\"parse_and_verify\",\"type_arguments\":[],\"arguments\":[{\"Input\":0},{\"Input\":1},{\"Input\":2}]}},{\"MoveCall\":{\"package\":\"0x04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"pyth\",\"function\":\"create_authenticated_price_infos_using_accumulator\",\"type_arguments\":[],\"arguments\":[{\"Input\":3},{\"Input\":4},{\"NestedResult\":[0,0]},{\"Input\":2}]}},{\"SplitCoins\":[\"GasCoin\",[{\"Input\":5},{\"Input\":6},{\"Input\":7}]]},{\"MoveCall\":{\"package\":\"0x04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"pyth\",\"function\":\"update_single_price_feed\",\"type_arguments\":[],\"arguments\":[{\"Input\":3},{\"NestedResult\":[1,0]},{\"Input\":8},{\"NestedResult\":[2,0]},{\"Input\":2}]}},{\"MoveCall\":{\"package\":\"0x04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"pyth\",\"function\":\"update_single_price_feed\",\"type_arguments\":[],\"arguments\":[{\"Input\":3},{\"NestedResult\":[3,0]},{\"Input\":9},{\"NestedResult\":[2,1]},{\"Input\":2}]}},{\"MoveCall\":{\"package\":\"0x04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"pyth\",\"function\":\"update_single_price_feed\",\"type_arguments\":[],\"arguments\":[{\"Input\":3},{\"NestedResult\":[4,0]},{\"Input\":10},{\"NestedResult\":[2,2]},{\"Input\":2}]}},{\"MoveCall\":{\"package\":\"0x04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"hot_potato_vector\",\"function\":\"destroy\",\"type_arguments\":[{\"struct\":{\"address\":\"04e20ddf36af412a4096f9014f4a565af9e812db9a05cc40254846cf6ed0ad91\",\"module\":\"price_info\",\"name\":\"PriceInfo\",\"type_args\":[]}}],\"arguments\":[{\"NestedResult\":[5,0]}]}}]}}}";

fn main() {
    let mut output_path = std::path::PathBuf::from(".");
    output_path.set_file_name("out.html");
    generate_html_from_json(DATA, output_path, "mainnet");
}
