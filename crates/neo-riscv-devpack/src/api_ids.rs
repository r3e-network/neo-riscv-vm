//! Numeric syscall (interop) ids for every `System.*` service the host exposes.
//!
//! Each constant is the 32-bit FNV-1a interop hash of the corresponding
//! `System.<Area>.<Method>` name (computed by
//! [`neo_riscv_abi::interop_hash`]). The guest passes one of these as the `api`
//! argument to `host_call`, and the host bridge dispatches it to the canonical
//! NEO handler. These ids must stay aligned with the names registered in
//! `neo-riscv-host::bridge`.
//!
//! [`neo_riscv_abi::interop_hash`]: neo_riscv_abi::interop_hash

// System.Storage
/// `System.Storage.GetContext` — get the current contract's storage context.
pub const STORAGE_GET_CONTEXT: u32 = 0xce67_f69b;
/// `System.Storage.GetReadOnlyContext` — get a read-only storage context.
pub const STORAGE_GET_READONLY_CONTEXT: u32 = 0xe26b_b4f6;
/// `System.Storage.AsReadOnly` — convert a storage context to read-only.
pub const STORAGE_AS_READ_ONLY: u32 = 0xe9bf_4c76;
/// `System.Storage.Get` — read a value from storage.
pub const STORAGE_GET: u32 = 0x31e8_5d92;
/// `System.Storage.Put` — write a key/value pair to storage.
pub const STORAGE_PUT: u32 = 0x8418_3fe6;
/// `System.Storage.Delete` — delete a key from storage.
pub const STORAGE_DELETE: u32 = 0xedc5_582f;
/// `System.Storage.Find` — seek over keys matching a prefix.
pub const STORAGE_FIND: u32 = 0x9ab8_30df;
/// `System.Storage.Local.Get` — read from the current contract's private storage.
pub const STORAGE_LOCAL_GET: u32 = 0xe85e_8dd5;
/// `System.Storage.Local.Put` — write to the current contract's private storage.
pub const STORAGE_LOCAL_PUT: u32 = 0x0ae3_0c39;
/// `System.Storage.Local.Delete` — delete from the current contract's private storage.
pub const STORAGE_LOCAL_DELETE: u32 = 0x94f5_5475;
/// `System.Storage.Local.Find` — seek over keys matching a prefix in private storage.
pub const STORAGE_LOCAL_FIND: u32 = 0xf352_7607;

// System.Contract
/// `System.Contract.Call` — invoke a method on another contract.
pub const CONTRACT_CALL: u32 = 0x525b_7d62;
/// `System.Contract.Create` — deploy a new contract.
pub const CONTRACT_CREATE: u32 = 0x852c_35ce;
/// `System.Contract.Update` — update an existing contract's script/manifest.
pub const CONTRACT_UPDATE: u32 = 0x1d33_c631;
/// `System.Contract.GetCallFlags` — get the call flags of the current invocation.
pub const CONTRACT_GET_CALL_FLAGS: u32 = 0x813a_da95;
/// `System.Contract.CreateStandardAccount` — derive a standard account from a public key.
pub const CONTRACT_CREATE_STANDARD_ACCOUNT: u32 = 0x0287_99cf;
/// `System.Contract.CreateMultisigAccount` — derive a multisig account from a public key list.
pub const CONTRACT_CREATE_MULTISIG_ACCOUNT: u32 = 0x09e9_336a;
/// `System.Contract.NativeOnPersist` — native on-persist hook (native contracts only).
pub const CONTRACT_NATIVE_ON_PERSIST: u32 = 0x93bc_db2e;
/// `System.Contract.NativePostPersist` — native post-persist hook (native contracts only).
pub const CONTRACT_NATIVE_POST_PERSIST: u32 = 0x165d_a144;

// System.Runtime
/// `System.Runtime.Platform` — return the platform identifier (NEO).
pub const RUNTIME_PLATFORM: u32 = 0xf6fc_79b2;
/// `System.Runtime.GetTrigger` — return the current trigger type.
pub const RUNTIME_GET_TRIGGER: u32 = 0xa038_7de9;
/// `System.Runtime.GetNetwork` — return the network magic number.
pub const RUNTIME_GET_NETWORK: u32 = 0xe0a0_fbc5;
/// `System.Runtime.GetAddressVersion` — return the address version byte.
pub const RUNTIME_GET_ADDRESS_VERSION: u32 = 0xdc92_494c;
/// `System.Runtime.GetScriptContainer` — return the current transaction/container.
pub const RUNTIME_GET_SCRIPT_CONTAINER: u32 = 0x3008_512d;
/// `System.Runtime.GetExecutingScriptHash` — hash of the currently executing contract.
pub const RUNTIME_GET_EXECUTING_SCRIPT_HASH: u32 = 0x74a8_fedb;
/// `System.Runtime.GetCallingScriptHash` — hash of the calling contract.
pub const RUNTIME_GET_CALLING_SCRIPT_HASH: u32 = 0x3c6e_5339;
/// `System.Runtime.GetEntryScriptHash` — hash of the entry-point contract.
pub const RUNTIME_GET_ENTRY_SCRIPT_HASH: u32 = 0x38e2_b4f9;
/// `System.Runtime.GetTime` — return the current block timestamp.
pub const RUNTIME_GET_TIME: u32 = 0x0388_c3b7;
/// `System.Runtime.GetInvocationCounter` — depth of the current call chain.
pub const RUNTIME_GET_INVOCATION_COUNTER: u32 = 0x4311_2784;
/// `System.Runtime.GasLeft` — remaining gas for the current invocation.
pub const RUNTIME_GAS_LEFT: u32 = 0xced8_8814;
/// `System.Runtime.GetRandom` — return a deterministic per-block random number.
pub const RUNTIME_GET_RANDOM: u32 = 0x28a9_de6b;
/// `System.Runtime.CurrentSigners` — return the current transaction's signers.
pub const RUNTIME_CURRENT_SIGNERS: u32 = 0x8b18_f1ac;
/// `System.Runtime.CheckWitness` — verify a hash has signed the transaction.
pub const RUNTIME_CHECK_WITNESS: u32 = 0x8cec_27f8;
/// `System.Runtime.Notify` — emit a runtime notification event.
pub const RUNTIME_NOTIFY: u32 = 0x616f_0195;
/// `System.Runtime.Log` — emit a log message.
pub const RUNTIME_LOG: u32 = 0x9647_e7cf;
/// `System.Runtime.GetNotifications` — retrieve notifications emitted by a contract.
pub const RUNTIME_GET_NOTIFICATIONS: u32 = 0xf135_4327;
/// `System.Runtime.BurnGas` — explicitly burn gas.
pub const RUNTIME_BURN_GAS: u32 = 0xbc8c_5ac3;
/// `System.Runtime.LoadScript` — dynamically load and execute a script.
pub const RUNTIME_LOAD_SCRIPT: u32 = 0x8f80_0cb3;

// System.Crypto
/// `System.Crypto.CheckSig` — verify a single signature.
pub const CRYPTO_CHECK_SIG: u32 = 0x27b3_e756;
/// `System.Crypto.CheckMultisig` — verify an m-of-n multisignature.
pub const CRYPTO_CHECK_MULTISIG: u32 = 0x3adc_d09e;

// System.Iterator
/// `System.Iterator.Next` — advance an iterator to the next element.
pub const ITERATOR_NEXT: u32 = 0x9ced_089c;
/// `System.Iterator.Value` — get the current element of an iterator.
pub const ITERATOR_VALUE: u32 = 0x1dbf_54f3;

// Backward-compatible aliases
/// Backward-compatible alias for [`CRYPTO_CHECK_SIG`].
pub const CRYPTO_VERIFY_SIGNATURE: u32 = CRYPTO_CHECK_SIG;
