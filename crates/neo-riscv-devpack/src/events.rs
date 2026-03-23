use neo_riscv_abi::StackValue;

use crate::syscalls::runtime_notify;

pub fn notify(event_name: &str, state: &[u8]) {
    let payload = [StackValue::ByteString(state.to_vec())];
    runtime_notify(event_name, &payload);
}
