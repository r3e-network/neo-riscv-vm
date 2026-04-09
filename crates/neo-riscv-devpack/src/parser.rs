use alloc::{format, string::String, vec::Vec};
use neo_riscv_abi::StackValue;

/// Parse a raw byte slice as a StackValue using the fast codec.
pub fn parse_stack_value(data: &[u8]) -> Result<StackValue, &'static str> {
    let stack = neo_riscv_abi::fast_codec::decode_stack(data)?;
    stack.into_iter().next().ok_or("empty stack")
}

/// Parse a raw byte slice as a UTF-8 string from a ByteString StackValue.
pub fn parse_string_result(data: &[u8]) -> Result<String, &'static str> {
    let value = parse_stack_value(data)?;
    match value {
        StackValue::ByteString(bytes) => String::from_utf8(bytes).map_err(|_| "invalid utf-8"),
        _ => Err("expected ByteString"),
    }
}

/// Parse a raw byte slice as an i64 from an Integer StackValue.
pub fn parse_int_result(data: &[u8]) -> Result<i64, &'static str> {
    let value = parse_stack_value(data)?;
    match value {
        StackValue::Integer(i) => Ok(i),
        _ => Err("expected Integer"),
    }
}

/// Format a StackValue as a human-readable debug string.
pub fn format_stack_value(value: &StackValue) -> String {
    match value {
        StackValue::Integer(i) => format!("Integer({i})"),
        StackValue::Boolean(b) => format!("Boolean({b})"),
        StackValue::ByteString(bytes) => {
            if let Ok(s) = core::str::from_utf8(bytes) {
                format!("ByteString(\"{s}\")")
            } else {
                format!("ByteString({bytes:?})")
            }
        }
        StackValue::Null => "Null".into(),
        StackValue::Array(items) => {
            let inner: Vec<String> = items.iter().map(format_stack_value).collect();
            format!("Array([{}])", inner.join(", "))
        }
        _ => format!("{value:?}"),
    }
}
