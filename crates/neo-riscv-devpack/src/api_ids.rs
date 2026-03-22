// Syscall API IDs derived from `neo_riscv_abi::interop_hash`.
// These must stay aligned with the bridge interop names.

// System.Storage
pub const STORAGE_GET: u32 = 0x31e8_5d92; // System.Storage.Get
pub const STORAGE_PUT: u32 = 0x8418_3fe6; // System.Storage.Put
pub const STORAGE_DELETE: u32 = 0xedc5_582f; // System.Storage.Delete

// System.Contract
pub const CONTRACT_CALL: u32 = 0x525b_7d62; // System.Contract.Call
pub const CONTRACT_CREATE: u32 = 0x852c_35ce; // System.Contract.Create
pub const CONTRACT_UPDATE: u32 = 0x1d33_c631; // System.Contract.Update

// System.Runtime
pub const RUNTIME_NOTIFY: u32 = 0x616f_0195; // System.Runtime.Notify
pub const RUNTIME_LOG: u32 = 0x9647_e7cf; // System.Runtime.Log
pub const RUNTIME_CHECK_WITNESS: u32 = 0x8cec_27f8; // System.Runtime.CheckWitness

// System.Crypto
pub const CRYPTO_VERIFY_SIGNATURE: u32 = 0x27b3_e756; // System.Crypto.CheckSig
