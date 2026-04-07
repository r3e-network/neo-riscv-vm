use alloc::{string::String, vec, vec::Vec};
use core::convert::TryInto;

use neo_riscv_abi::StackValue;

use crate::{api_ids, ffi, syscalls::CALL_FLAGS_ALL};

pub mod contract_management;
pub mod crypto_lib;
pub mod gas_token;
pub mod ledger;
pub mod neo_token;
pub mod notary;
pub mod oracle;
pub mod policy;
pub mod role_management;
pub mod std_lib;
pub mod treasury;

pub(crate) const CALL_FLAGS_READ_ONLY: u8 = 0x05;

/// Build the evaluation stack for a `System.Contract.Call` host invocation
/// targeting a native contract.
///
/// This version takes a fixed `&[u8; 20]` hash, which guarantees at compile
/// time that the hash length matches a Neo `UInt160`.  For general-purpose
/// calls where the hash comes from dynamic input, see
/// [`crate::syscalls::build_contract_call_stack`].
pub(crate) fn build_contract_call_stack(
    hash: &[u8; 20],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> Vec<StackValue> {
    vec![
        StackValue::Array(args.to_vec()),
        StackValue::Integer(i64::from(call_flags)),
        StackValue::ByteString(method.as_bytes().to_vec()),
        StackValue::ByteString(hash.to_vec()),
    ]
}

pub(crate) fn call_native(
    hash: &[u8; 20],
    method: &str,
    args: &[StackValue],
) -> Option<StackValue> {
    call_native_with_flags(hash, method, CALL_FLAGS_ALL, args)
}

pub(crate) fn call_native_read_only(
    hash: &[u8; 20],
    method: &str,
    args: &[StackValue],
) -> Option<StackValue> {
    call_native_with_flags(hash, method, CALL_FLAGS_READ_ONLY, args)
}

pub(crate) fn call_native_with_flags(
    hash: &[u8; 20],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> Option<StackValue> {
    ffi::invoke_host_call(
        api_ids::CONTRACT_CALL,
        &build_contract_call_stack(hash, method, call_flags, args),
    )
    .ok()
    .and_then(|result| result.into_iter().next())
}

pub(crate) fn stack_item_as_bool(item: &StackValue) -> Option<bool> {
    match item {
        StackValue::Boolean(value) => Some(*value),
        StackValue::Integer(value) => Some(*value != 0),
        StackValue::BigInteger(bytes) => Some(big_integer_to_i64(bytes)? != 0),
        _ => None,
    }
}

pub(crate) fn stack_item_as_i64(item: &StackValue) -> Option<i64> {
    match item {
        StackValue::Integer(value) => Some(*value),
        StackValue::BigInteger(bytes) => big_integer_to_i64(bytes),
        _ => None,
    }
}

pub(crate) fn stack_item_as_u32(item: &StackValue) -> Option<u32> {
    stack_item_as_i64(item)?.try_into().ok()
}

pub(crate) fn stack_item_as_u8(item: &StackValue) -> Option<u8> {
    stack_item_as_i64(item)?.try_into().ok()
}

pub(crate) fn stack_item_as_bytes(item: &StackValue) -> Option<Vec<u8>> {
    match item {
        StackValue::ByteString(bytes) => Some(bytes.clone()),
        _ => None,
    }
}

pub(crate) fn stack_item_as_fixed_bytes<const N: usize>(item: &StackValue) -> Option<[u8; N]> {
    let bytes = stack_item_as_bytes(item)?;
    bytes.try_into().ok()
}

pub(crate) fn stack_item_as_string(item: &StackValue) -> Option<String> {
    String::from_utf8(stack_item_as_bytes(item)?).ok()
}

pub(crate) fn stack_item_into_items(item: StackValue) -> Option<Vec<StackValue>> {
    match item {
        StackValue::Array(items) | StackValue::Struct(items) => Some(items),
        _ => None,
    }
}

fn big_integer_to_i64(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() {
        return Some(0);
    }

    if bytes.len() <= 8 {
        let negative = bytes.last().copied().unwrap_or_default() & 0x80 != 0;
        let mut extended = [if negative { 0xff } else { 0x00 }; 8];
        extended[..bytes.len()].copy_from_slice(bytes);
        return Some(i64::from_le_bytes(extended));
    }

    if bytes.len() > 16 {
        return None;
    }

    let negative = bytes.last().copied().unwrap_or_default() & 0x80 != 0;
    let mut extended = [if negative { 0xff } else { 0x00 }; 16];
    extended[..bytes.len()].copy_from_slice(bytes);
    i128::from_le_bytes(extended).try_into().ok()
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use neo_riscv_abi::StackValue;

    use super::{
        build_contract_call_stack, stack_item_as_bool, stack_item_as_fixed_bytes,
        stack_item_as_i64, CALL_FLAGS_ALL,
    };

    #[test]
    fn build_contract_call_stack_matches_bridge_shape() {
        let hash = [0x55; 20];
        let args = [StackValue::Interop(2), StackValue::Interop(1)];

        let stack = build_contract_call_stack(&hash, "bls12381Add", CALL_FLAGS_ALL, &args);

        assert_eq!(
            stack,
            vec![
                StackValue::Array(args.to_vec()),
                StackValue::Integer(i64::from(CALL_FLAGS_ALL)),
                StackValue::ByteString(b"bls12381Add".to_vec()),
                StackValue::ByteString(hash.to_vec()),
            ]
        );
    }

    #[test]
    fn extractors_handle_common_values_and_defaults() {
        assert_eq!(stack_item_as_i64(&StackValue::Integer(42)), Some(42));
        assert_eq!(
            stack_item_as_i64(&StackValue::BigInteger(vec![0xff, 0x00])),
            Some(255)
        );
        assert_eq!(stack_item_as_i64(&StackValue::Null), None);

        assert_eq!(stack_item_as_bool(&StackValue::Boolean(true)), Some(true));
        assert_eq!(stack_item_as_bool(&StackValue::Integer(0)), Some(false));
        assert_eq!(stack_item_as_bool(&StackValue::Null), None);

        assert_eq!(
            stack_item_as_fixed_bytes::<4>(&StackValue::ByteString(vec![1, 2, 3, 4])),
            Some([1, 2, 3, 4])
        );
        assert_eq!(
            stack_item_as_fixed_bytes::<4>(&StackValue::ByteString(vec![1, 2, 3])),
            None
        );
    }
}
