use neo_riscv_abi::StackValue;

use crate::syscalls::runtime_notify;

/// Emit a notification event with raw byte payload.
pub fn notify(event_name: &str, state: &[u8]) {
    let payload = [StackValue::ByteString(state.to_vec())];
    runtime_notify(event_name, &payload);
}

/// Emit a notification event with a string message.
pub fn notify_string(event_name: &str, message: &str) {
    let payload = [StackValue::ByteString(message.as_bytes().to_vec())];
    runtime_notify(event_name, &payload);
}

/// Emit a notification event with an integer value.
pub fn notify_int(event_name: &str, value: i64) {
    let payload = [StackValue::Integer(value)];
    runtime_notify(event_name, &payload);
}

/// Emit a notification event with multiple StackValue items.
pub fn notify_args(event_name: &str, args: &[StackValue]) {
    runtime_notify(event_name, args);
}

/// Emit a notification event with a string key and integer value (common pattern).
pub fn notify_key_value(event_name: &str, key: &str, value: i64) {
    let payload = [
        StackValue::ByteString(key.as_bytes().to_vec()),
        StackValue::Integer(value),
    ];
    runtime_notify(event_name, &payload);
}

/// Emit a notification event with two string values.
pub fn notify_two_strings(event_name: &str, a: &str, b: &str) {
    let payload = [
        StackValue::ByteString(a.as_bytes().to_vec()),
        StackValue::ByteString(b.as_bytes().to_vec()),
    ];
    runtime_notify(event_name, &payload);
}
