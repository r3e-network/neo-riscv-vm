use neo_riscv_abi::StackValue;

use super::call_native;

// OracleContract native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const ORACLE_CONTRACT_HASH: [u8; 20] = [
    0x58, 0x87, 0x17, 0x11, 0x7e, 0x0a, 0xa8, 0x10, 0x72, 0xaf, 0xab, 0x71, 0xd2, 0xdd, 0x89, 0xfe,
    0x7c, 0x4b, 0x92, 0xfe,
];

// Oracle native contract bindings
pub fn oracle_request(
    url: &str,
    filter: &str,
    callback: &str,
    user_data: &[u8],
    gas_for_response: i64,
) {
    let args = [
        StackValue::ByteString(url.as_bytes().to_vec()),
        StackValue::ByteString(filter.as_bytes().to_vec()),
        StackValue::ByteString(callback.as_bytes().to_vec()),
        StackValue::ByteString(user_data.to_vec()),
        StackValue::Integer(gas_for_response),
    ];
    let _ = call_native(&ORACLE_CONTRACT_HASH, "request", &args);
}
