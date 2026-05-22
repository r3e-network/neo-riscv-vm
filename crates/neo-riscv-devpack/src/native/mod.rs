use alloc::{vec, vec::Vec};

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

#[cfg(test)]
mod tests {
    use alloc::vec;
    use neo_riscv_abi::StackValue;

    use super::{build_contract_call_stack, CALL_FLAGS_ALL};

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
}
