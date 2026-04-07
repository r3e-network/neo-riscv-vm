// Syscall API IDs derived from `neo_riscv_abi::interop_hash`.
// These must stay aligned with the bridge interop names.

// System.Storage
pub const STORAGE_GET_CONTEXT: u32 = 0xce67_f69b; // System.Storage.GetContext
pub const STORAGE_GET_READONLY_CONTEXT: u32 = 0xe26b_b4f6; // System.Storage.GetReadOnlyContext
pub const STORAGE_AS_READ_ONLY: u32 = 0xe9bf_4c76; // System.Storage.AsReadOnly
pub const STORAGE_GET: u32 = 0x31e8_5d92; // System.Storage.Get
pub const STORAGE_PUT: u32 = 0x8418_3fe6; // System.Storage.Put
pub const STORAGE_DELETE: u32 = 0xedc5_582f; // System.Storage.Delete
pub const STORAGE_FIND: u32 = 0x9ab8_30df; // System.Storage.Find
pub const STORAGE_LOCAL_GET: u32 = 0xe85e_8dd5; // System.Storage.Local.Get
pub const STORAGE_LOCAL_PUT: u32 = 0x0ae3_0c39; // System.Storage.Local.Put
pub const STORAGE_LOCAL_DELETE: u32 = 0x94f5_5475; // System.Storage.Local.Delete
pub const STORAGE_LOCAL_FIND: u32 = 0xf352_7607; // System.Storage.Local.Find

// System.Contract
pub const CONTRACT_CALL: u32 = 0x525b_7d62; // System.Contract.Call
pub const CONTRACT_CREATE: u32 = 0x852c_35ce; // System.Contract.Create
pub const CONTRACT_UPDATE: u32 = 0x1d33_c631; // System.Contract.Update
pub const CONTRACT_GET_CALL_FLAGS: u32 = 0x813a_da95; // System.Contract.GetCallFlags
pub const CONTRACT_CREATE_STANDARD_ACCOUNT: u32 = 0x0287_99cf; // System.Contract.CreateStandardAccount
pub const CONTRACT_CREATE_MULTISIG_ACCOUNT: u32 = 0x09e9_336a; // System.Contract.CreateMultisigAccount
pub const CONTRACT_NATIVE_ON_PERSIST: u32 = 0x93bc_db2e; // System.Contract.NativeOnPersist
pub const CONTRACT_NATIVE_POST_PERSIST: u32 = 0x165d_a144; // System.Contract.NativePostPersist

// System.Runtime
pub const RUNTIME_PLATFORM: u32 = 0xf6fc_79b2; // System.Runtime.Platform
pub const RUNTIME_GET_TRIGGER: u32 = 0xa038_7de9; // System.Runtime.GetTrigger
pub const RUNTIME_GET_NETWORK: u32 = 0xe0a0_fbc5; // System.Runtime.GetNetwork
pub const RUNTIME_GET_ADDRESS_VERSION: u32 = 0xdc92_494c; // System.Runtime.GetAddressVersion
pub const RUNTIME_GET_SCRIPT_CONTAINER: u32 = 0x3008_512d; // System.Runtime.GetScriptContainer
pub const RUNTIME_GET_EXECUTING_SCRIPT_HASH: u32 = 0x74a8_fedb; // System.Runtime.GetExecutingScriptHash
pub const RUNTIME_GET_CALLING_SCRIPT_HASH: u32 = 0x3c6e_5339; // System.Runtime.GetCallingScriptHash
pub const RUNTIME_GET_ENTRY_SCRIPT_HASH: u32 = 0x38e2_b4f9; // System.Runtime.GetEntryScriptHash
pub const RUNTIME_GET_TIME: u32 = 0x0388_c3b7; // System.Runtime.GetTime
pub const RUNTIME_GET_INVOCATION_COUNTER: u32 = 0x4311_2784; // System.Runtime.GetInvocationCounter
pub const RUNTIME_GAS_LEFT: u32 = 0xced8_8814; // System.Runtime.GasLeft
pub const RUNTIME_GET_RANDOM: u32 = 0x28a9_de6b; // System.Runtime.GetRandom
pub const RUNTIME_CURRENT_SIGNERS: u32 = 0x8b18_f1ac; // System.Runtime.CurrentSigners
pub const RUNTIME_CHECK_WITNESS: u32 = 0x8cec_27f8; // System.Runtime.CheckWitness
pub const RUNTIME_NOTIFY: u32 = 0x616f_0195; // System.Runtime.Notify
pub const RUNTIME_LOG: u32 = 0x9647_e7cf; // System.Runtime.Log
pub const RUNTIME_GET_NOTIFICATIONS: u32 = 0xf135_4327; // System.Runtime.GetNotifications
pub const RUNTIME_BURN_GAS: u32 = 0xbc8c_5ac3; // System.Runtime.BurnGas
pub const RUNTIME_LOAD_SCRIPT: u32 = 0x8f80_0cb3; // System.Runtime.LoadScript

// System.Crypto
pub const CRYPTO_CHECK_SIG: u32 = 0x27b3_e756; // System.Crypto.CheckSig
pub const CRYPTO_CHECK_MULTISIG: u32 = 0x3adc_d09e; // System.Crypto.CheckMultisig

// System.Iterator
pub const ITERATOR_NEXT: u32 = 0x9ced_089c; // System.Iterator.Next
pub const ITERATOR_VALUE: u32 = 0x1dbf_54f3; // System.Iterator.Value

// Backward-compatible aliases
pub const CRYPTO_VERIFY_SIGNATURE: u32 = CRYPTO_CHECK_SIG;
