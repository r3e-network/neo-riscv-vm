use alloc::{string::String, vec::Vec};
use neo_riscv_abi::StackValue;

/// Encode a slice of strings as a StackValue::Array of ByteString values.
/// This is useful for passing string parameters to contract calls.
pub fn encode_string_params(params: &[&str]) -> StackValue {
    let items: Vec<StackValue> = params
        .iter()
        .map(|s| StackValue::ByteString(s.as_bytes().to_vec()))
        .collect();
    StackValue::Array(items)
}

/// Encode integer parameters as a StackValue::Array of Integer values.
pub fn encode_int_params(params: &[i64]) -> StackValue {
    let items: Vec<StackValue> = params
        .iter()
        .map(|&i| StackValue::Integer(i))
        .collect();
    StackValue::Array(items)
}

/// Decode a StackValue::ByteString result to a UTF-8 String.
pub fn decode_string_result(value: &StackValue) -> Option<String> {
    match value {
        StackValue::ByteString(bytes) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

/// Decode a StackValue::Integer result to i64.
pub fn decode_int_result(value: &StackValue) -> Option<i64> {
    match value {
        StackValue::Integer(i) => Some(*i),
        _ => None,
    }
}

/// Decode a StackValue::Boolean result to bool.
pub fn decode_bool_result(value: &StackValue) -> Option<bool> {
    match value {
        StackValue::Boolean(b) => Some(*b),
        StackValue::Integer(i) => Some(*i != 0),
        _ => None,
    }
}

/// Encode a single byte array as a StackValue::ByteString.
pub fn encode_bytes(data: &[u8]) -> StackValue {
    StackValue::ByteString(data.to_vec())
}

/// Decode a StackValue::ByteString result to raw bytes.
pub fn decode_bytes_result(value: &StackValue) -> Option<Vec<u8>> {
    match value {
        StackValue::ByteString(bytes) => Some(bytes.clone()),
        _ => None,
    }
}
