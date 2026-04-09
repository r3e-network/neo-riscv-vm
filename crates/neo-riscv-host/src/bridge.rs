use crate::{pricing::charge_opcode, HostCallbackResult, RuntimeContext};
use neo_riscv_abi::{callback_codec, fast_codec};
use neo_riscv_guest::SyscallProvider;
use polkavm::Linker;
use std::ffi::c_void;

pub(crate) type GuestTrace = Option<(u32, Vec<u8>)>;
type HostCallbackOutcome = Result<HostCallbackResult, String>;
type CallbackInvokeFn = unsafe fn(
    *mut c_void,
    u32,
    usize,
    RuntimeContext,
    &[neo_riscv_abi::StackValue],
) -> HostCallbackOutcome;

pub(crate) fn register_host_functions(
    linker: &mut Linker<ClosureHost, core::convert::Infallible>,
) -> Result<(), String> {
    linker
        .define_typed("host_on_instruction", host_on_instruction_import)
        .map_err(|e| e.to_string())?;
    linker
        .define_typed("host_call", host_call_import)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn host_on_instruction_import(caller: polkavm::Caller<ClosureHost>, opcode: u32) -> u32 {
    let host = caller.user_data;
    host.last_opcode = Some(opcode as u8);
    host.opcode_count = host.opcode_count.saturating_add(1);
    match charge_opcode(&mut host.context, &mut host.fee_consumed_pico, opcode as u8) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

fn host_call_import(
    caller: polkavm::Caller<ClosureHost>,
    api: u32,
    ip: u32,
    stack_ptr: u32,
    stack_len: u32,
    result_ptr: u32,
    result_cap: u32,
) -> u32 {
    let host = caller.user_data;
    host.last_host_call_stage = 1;
    host.syscall_count = host.syscall_count.saturating_add(1);
    host.last_api = Some(api);
    host.last_ip = Some(ip);
    host.last_stack_len = Some(stack_len);
    host.last_result_cap = Some(result_cap);

    host.callback_read_buf.resize(stack_len as usize, 0);
    host.last_host_call_stage = 2;
    if let Err(e) = caller
        .instance
        .read_memory_into(stack_ptr, &mut host.callback_read_buf[..])
    {
        eprintln!(
            "[neo-riscv-host] host_call(api={api}): read_memory_into failed at ptr=0x{stack_ptr:08x} len={stack_len}: {e}"
        );
        return 0;
    }

    let stack: Vec<neo_riscv_abi::StackValue> = match fast_codec::decode_stack(
        &host.callback_read_buf,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                    "[neo-riscv-host] host_call(api={api}): fast_codec deserialization failed (len={stack_len}): {e}"
                );
            return 0;
        }
    };

    host.last_host_call_stage = 3;
    let result = host.invoke(api, ip as usize, &stack);

    host.last_host_call_stage = 4;
    let bytes = match result {
        Ok(res) => {
            let payload: Result<Vec<neo_riscv_abi::StackValue>, String> = Ok(res.stack);
            callback_codec::encode_stack_result(&payload)
        }
        Err(ref error) => {
            let payload: Result<Vec<neo_riscv_abi::StackValue>, String> = Err(error.clone());
            callback_codec::encode_stack_result(&payload)
        }
    };
    if bytes.len() > result_cap as usize {
        eprintln!(
            "[neo-riscv-host] host_call(api={api}): result buffer too small (need {} bytes, cap={result_cap})",
            bytes.len()
        );
        return 0;
    }
    host.last_host_call_stage = 5;
    if let Err(e) = caller.instance.write_memory(result_ptr, &bytes) {
        eprintln!(
            "[neo-riscv-host] host_call(api={api}): write_memory failed at ptr=0x{result_ptr:08x}: {e}"
        );
        return 0;
    }
    host.last_host_call_stage = 6;
    bytes.len() as u32
}

pub(crate) struct ClosureHost {
    pub(crate) context: RuntimeContext,
    pub(crate) fee_consumed_pico: i64,
    pub(crate) last_opcode: Option<u8>,
    pub(crate) opcode_count: u64,
    pub(crate) syscall_count: u32,
    pub(crate) last_api: Option<u32>,
    pub(crate) last_ip: Option<u32>,
    pub(crate) last_stack_len: Option<u32>,
    pub(crate) last_result_cap: Option<u32>,
    pub(crate) last_host_call_stage: u32,
    pub(crate) callback_read_buf: Vec<u8>,
    callback_data: *mut c_void,
    callback_invoke: CallbackInvokeFn,
}

impl ClosureHost {
    pub(crate) fn new<F>(context: RuntimeContext, callback: &mut F) -> Self
    where
        F: FnMut(
            u32,
            usize,
            RuntimeContext,
            &[neo_riscv_abi::StackValue],
        ) -> Result<HostCallbackResult, String>,
    {
        Self {
            context,
            fee_consumed_pico: 0,
            last_opcode: None,
            opcode_count: 0,
            syscall_count: 0,
            last_api: None,
            last_ip: None,
            last_stack_len: None,
            last_result_cap: None,
            last_host_call_stage: 0,
            callback_read_buf: Vec::new(),
            callback_data: callback as *mut F as *mut c_void,
            callback_invoke: invoke_callback::<F>,
        }
    }

    fn invoke(
        &mut self,
        api: u32,
        ip: usize,
        stack: &[neo_riscv_abi::StackValue],
    ) -> HostCallbackOutcome {
        // SAFETY: callback_data was stored by ClosureHost::new() from a &mut F cast to *mut c_void.
        // ClosureHost does not outlive the &mut F borrow — all callers drop ClosureHost before the
        // enclosing function returns, ensuring the pointer remains valid for the lifetime of invoke().
        unsafe { (self.callback_invoke)(self.callback_data, api, ip, self.context, stack) }
    }
}

unsafe fn invoke_callback<F>(
    callback_data: *mut c_void,
    api: u32,
    ip: usize,
    context: RuntimeContext,
    stack: &[neo_riscv_abi::StackValue],
) -> HostCallbackOutcome
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    // SAFETY: callback_data points to a live &mut F stored by ClosureHost::new().
    // The caller (ClosureHost::invoke) guarantees the pointer is valid and uniquely borrowed.
    let callback = &mut *(callback_data as *mut F);
    callback(api, ip, context, stack)
}

pub(crate) fn read_guest_trace(
    instance: &mut polkavm::Instance<ClosureHost>,
    host: &mut ClosureHost,
) -> GuestTrace {
    let len = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_trace_res_len", ())
        .ok()?;
    let ptr = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_trace_res_head_ptr", ())
        .ok()?;
    let copy_len = core::cmp::min(len as usize, 32);
    let mut bytes = vec![0u8; copy_len];
    instance.read_memory_into(ptr, &mut bytes[..]).ok()?;
    Some((len, bytes))
}

pub(crate) fn read_guest_panic(
    instance: &mut polkavm::Instance<ClosureHost>,
    host: &mut ClosureHost,
) -> Option<String> {
    let len = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_panic_len", ())
        .ok()?;
    if len == 0 {
        return None;
    }
    let ptr = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_panic_ptr", ())
        .ok()?;
    if ptr == 0 {
        return None;
    }
    let mut bytes = vec![0u8; len as usize];
    instance.read_memory_into(ptr, &mut bytes[..]).ok()?;
    Some(String::from_utf8_lossy(&bytes).to_string())
}

pub(crate) fn read_guest_debug(
    instance: &mut polkavm::Instance<ClosureHost>,
    host: &mut ClosureHost,
) -> Option<Vec<u8>> {
    let len = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_debug_len", ())
        .ok()?;
    if len == 0 {
        return Some(Vec::new());
    }
    let ptr = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_debug_ptr", ())
        .ok()?;
    if ptr == 0 {
        return None;
    }
    let mut bytes = vec![0u8; len as usize];
    instance.read_memory_into(ptr, &mut bytes[..]).ok()?;
    Some(bytes)
}

pub(crate) fn read_pc_trace(
    instance: &mut polkavm::Instance<ClosureHost>,
    host: &mut ClosureHost,
) -> Option<Vec<u8>> {
    let len = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_pc_trace_len", ())
        .ok()?;
    if len == 0 {
        return Some(Vec::new());
    }
    let ptr = instance
        .call_typed_and_get_result::<u32, ()>(host, "get_pc_trace_ptr", ())
        .ok()?;
    if ptr == 0 {
        return None;
    }
    let mut bytes = vec![0u8; len as usize];
    instance.read_memory_into(ptr, &mut bytes[..]).ok()?;
    Some(bytes)
}

impl SyscallProvider for ClosureHost {
    fn on_instruction(&mut self, opcode: u8) -> Result<(), String> {
        self.last_opcode = Some(opcode);
        self.opcode_count = self.opcode_count.saturating_add(1);
        charge_opcode(&mut self.context, &mut self.fee_consumed_pico, opcode)
    }

    fn syscall(
        &mut self,
        api: u32,
        ip: usize,
        stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        let result = self.invoke(api, ip, stack)?;
        *stack = result.stack;
        Ok(())
    }
}
