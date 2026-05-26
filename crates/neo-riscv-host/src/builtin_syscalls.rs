use crate::{host_callback_result::HostCallbackResult, error::HostError};
use neo_riscv_abi::{interop_hash, StackValue};
use crate::runtime_context::RuntimeContext;

#[allow(clippy::if_same_then_else)]
pub(crate) fn builtin_host_callback(
    api: u32,
    context: RuntimeContext,
    _stack: &[StackValue],
) -> Result<HostCallbackResult, HostError> {
    // SAFETY: builtin mode is for development/testing only. In production,
    // the C# host provides real signature/authorization verification via the
    // FFI callback path. See `execute_script_with_host_and_stack_and_ip` for
    // the production execution path.
    #[cfg(not(debug_assertions))]
    {
        return Err(HostError::BuiltinDisabled);
    }
    #[allow(unused_variables)]
    let _guard = (api, context, _stack);

    // All builtin syscalls are 0-arg: they receive an empty stack and return [result].
    // The caller (invoke_syscall) handles popping consumed args and pushing results.

    // System.Runtime (zero-arg getters)
    if api == interop_hash("System.Runtime.Platform") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(b"NEO".to_vec())],
        })
    } else if api == interop_hash("System.Runtime.GetTrigger") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.trigger))],
        })
    } else if api == interop_hash("System.Runtime.GetNetwork") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.network))],
        })
    } else if api == interop_hash("System.Runtime.GetAddressVersion") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.address_version))],
        })
    } else if api == interop_hash("System.Runtime.GasLeft") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(context.gas_left)],
        })
    } else if api == interop_hash("System.Runtime.GetTime") {
        match context.timestamp {
            Some(timestamp) => Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(timestamp as i64)],
            }),
            None => Err(HostError::Other("GetTime requires a persisting block timestamp".into())),
        }
    } else if api == interop_hash("System.Runtime.GetScriptContainer") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Runtime.GetExecutingScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetCallingScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetEntryScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetInvocationCounter") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Runtime.GetRandom") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(42)],
        })
    } else if api == interop_hash("System.Runtime.CurrentSigners") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Runtime.GetNotifications") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Array(Vec::new())],
        })
    } else if api == interop_hash("System.Runtime.CheckWitness") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    } else if api == interop_hash("System.Runtime.BurnGas") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.Notify") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.Log") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.LoadScript") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    // System.Storage
    } else if api == interop_hash("System.Storage.GetContext") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.GetReadOnlyContext") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.AsReadOnly") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.Get") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Put") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Delete") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Find") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Local.Get") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Local.Put") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Local.Delete") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Local.Find") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    // System.Contract
    } else if api == interop_hash("System.Contract.Call") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Contract.Create") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Contract.Update") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Contract.GetCallFlags") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(0x0f)],
        })
    } else if api == interop_hash("System.Contract.CreateStandardAccount") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Contract.CreateMultisigAccount") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Contract.NativeOnPersist") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Contract.NativePostPersist") {
        Ok(HostCallbackResult { stack: vec![] })
    // System.Crypto
    } else if api == interop_hash("System.Crypto.CheckSig") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    } else if api == interop_hash("System.Crypto.CheckMultisig") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    // System.Iterator
    } else if api == interop_hash("System.Iterator.Next") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(false)],
        })
    } else if api == interop_hash("System.Iterator.Value") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else {
        Err(HostError::Other(format!("unsupported syscall 0x{api:08x}")))
    }
}
