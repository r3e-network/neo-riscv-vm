//! Neo RISC-V runtime library for C#-compiled smart contracts.
//!
//! This crate provides the `Context` struct and supporting types used by
//! generated Rust code emitted from the C# to RISC-V compiler backend.
//! The compiler generates calls into `Context` methods for stack operations,
//! variable access, arithmetic, and syscalls.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

pub mod arithmetic;
pub mod collections;
pub mod comparison;
pub mod conversion;
pub mod memory;
pub mod stack_value;
pub mod strings;

// Provide C memory intrinsics for targets where compiler_builtins doesn't
// export them as #[no_mangle] symbols (e.g. polkavm with ilp32e ABI).
#[cfg(not(feature = "std"))]
mod mem_intrinsics;

pub use stack_value::StackValue;

use neo_riscv_abi::{ExecutionResult, StackValue as AbiStackValue, VmState};

/// A try/catch/finally exception frame for the compiled state machine.
#[derive(Debug, Clone)]
struct TryFrame {
    /// Byte offset of the catch block (0 = no catch).
    catch_pc: i32,
    /// Byte offset of the finally block (0 = no finally).
    finally_pc: i32,
    /// Whether this frame has already caught an exception.
    caught: bool,
    /// Whether we are currently executing the finally block.
    in_finally: bool,
    /// Saved continuation offset after the finally block completes.
    end_pc: i32,
}

/// Signature of a syscall bridge function.
///
/// When set, `Context::syscall(hash)` delegates to this function, which is
/// responsible for marshaling arguments, calling the host, and pushing results.
pub type SyscallBridgeFn = fn(&mut Context, u32);

/// Global syscall bridge function pointer.
///
/// SAFETY: Written once at startup before any syscall fires. The PolkaVM guest
/// is single-threaded, so there is no data race.
static mut SYSCALL_BRIDGE: Option<SyscallBridgeFn> = None;

/// Register a syscall bridge that `Context::syscall()` will delegate to.
///
/// This is called by the contract harness at the start of `execute()`.
pub fn set_syscall_bridge(f: SyscallBridgeFn) {
    unsafe {
        SYSCALL_BRIDGE = Some(f);
    }
}

/// Execution context for a compiled smart contract.
///
/// Mirrors NeoVM execution semantics: an evaluation stack, local/argument slots,
/// static fields, and fault handling.
pub struct Context {
    /// Evaluation stack.
    pub stack: Vec<StackValue>,
    /// Local variable slots (initialized by `init_slot`).
    pub locals: Vec<StackValue>,
    /// Argument slots (populated from the evaluation stack by `init_slot`).
    pub args: Vec<StackValue>,
    /// Contract-level static fields.
    pub static_fields: Vec<StackValue>,
    /// Fault message, set when the contract aborts.
    pub fault_message: Option<String>,
    /// Current execution state.
    pub state: VmState,
    /// Exception handling try-block stack.
    try_stack: Vec<TryFrame>,
    /// Pending exception message (set by throw/abort/assert when try frames exist).
    pending_error: Option<String>,
    /// Call stack tracking return offsets for CALL/RET.
    /// Separate from the eval stack to avoid conflicts when entry-point
    /// methods (called via dispatch, not NeoVM CALL) have no return address.
    call_stack: Vec<i32>,
}

impl Context {
    /// Creates a new context from ABI stack values (the initial evaluation stack
    /// provided by the host).
    #[must_use]
    pub fn from_abi_stack(abi_stack: Vec<AbiStackValue>) -> Self {
        let stack = abi_stack.iter().map(StackValue::from_abi).collect();
        Self {
            stack,
            locals: Vec::new(),
            args: Vec::new(),
            static_fields: Vec::new(),
            fault_message: None,
            state: VmState::Halt,
            try_stack: Vec::new(),
            pending_error: None,
            call_stack: Vec::new(),
        }
    }

    /// Initializes local and argument slots.
    ///
    /// Creates `local_count` local slots (initialized to `Null`), and pops
    /// `arg_count` values from the evaluation stack into the argument slots.
    /// Arguments are popped in reverse order so that arg[0] is the value
    /// that was pushed first (deepest on the stack).
    pub fn init_slot(&mut self, local_count: usize, arg_count: usize) {
        self.locals = vec![StackValue::Null; local_count];
        self.args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            let val = self.pop();
            self.args.push(val);
        }
        self.args.reverse();
    }

    /// Initialize static slot only (NeoVM INITSSLOT).
    pub fn init_sslot(&mut self, count: usize) {
        if self.static_fields.len() < count {
            self.static_fields.resize(count, StackValue::Null);
        }
    }

    /// Sets the VM to fault state with the given message.
    pub fn fault(&mut self, msg: &str) {
        self.state = VmState::Fault;
        self.fault_message = Some(msg.to_string());
    }

    /// Returns true if the VM is in fault state.
    pub fn is_faulted(&self) -> bool {
        self.state == VmState::Fault
    }

    /// Converts the context into an `ExecutionResult` for return to the host.
    #[must_use]
    pub fn to_execution_result(self, fee_consumed_pico: i64) -> ExecutionResult {
        let stack = self.stack.iter().map(StackValue::to_abi).collect();
        ExecutionResult {
            fee_consumed_pico,
            state: self.state,
            stack,
            fault_message: self.fault_message,
        }
    }

    // ---------------------------------------------------------------
    // Stack operations
    // ---------------------------------------------------------------

    /// Pushes a value onto the evaluation stack.
    pub fn push(&mut self, value: StackValue) {
        self.stack.push(value);
    }

    /// Pushes an integer onto the evaluation stack.
    pub fn push_int(&mut self, v: i64) {
        self.stack.push(StackValue::Integer(v));
    }

    /// Pushes a boolean onto the evaluation stack.
    pub fn push_bool(&mut self, v: bool) {
        self.stack.push(StackValue::Boolean(v));
    }

    /// Pushes a byte string onto the evaluation stack.
    pub fn push_bytes(&mut self, v: &[u8]) {
        self.stack.push(StackValue::ByteString(v.to_vec()));
    }

    /// Pushes null onto the evaluation stack.
    pub fn push_null(&mut self) {
        self.stack.push(StackValue::Null);
    }

    /// Pops the top value from the evaluation stack.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    #[must_use]
    pub fn pop(&mut self) -> StackValue {
        self.stack
            .pop()
            .expect("stack underflow: pop on empty stack")
    }

    /// Pops and discards the top value from the evaluation stack.
    pub fn drop(&mut self) {
        let _ = self.pop();
    }

    /// Duplicates the top value on the evaluation stack.
    pub fn dup(&mut self) {
        let top = self
            .stack
            .last()
            .expect("stack underflow: dup on empty stack")
            .clone();
        self.stack.push(top);
    }

    /// Swaps the top two values on the evaluation stack.
    pub fn swap(&mut self) {
        let len = self.stack.len();
        assert!(len >= 2, "stack underflow: swap requires at least 2 items");
        self.stack.swap(len - 1, len - 2);
    }

    /// Removes the second-to-top item from the stack.
    pub fn nip(&mut self) {
        let len = self.stack.len();
        assert!(len >= 2, "stack underflow: nip requires at least 2 items");
        self.stack.remove(len - 2);
    }

    /// Pops an integer n, then removes the item at position n from the top.
    pub fn xdrop(&mut self) {
        let n = self.pop_integer();
        if n < 0 {
            self.fault("XDROP: negative index");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let n = n as usize;
        let len = self.stack.len();
        if n >= len {
            self.fault("XDROP: index out of range");
            return;
        }
        self.stack.remove(len - 1 - n);
    }

    /// Copies the second-to-top item and pushes it on top.
    pub fn over(&mut self) {
        let len = self.stack.len();
        assert!(len >= 2, "stack underflow: over requires at least 2 items");
        let val = self.stack[len - 2].clone();
        self.stack.push(val);
    }

    /// Pops an integer n from the stack, then copies the item at depth n and pushes it.
    pub fn pick(&mut self) {
        let n = self.pop_integer();
        #[allow(clippy::cast_sign_loss)]
        let n = n as usize;
        let len = self.stack.len();
        if n >= len {
            self.fault(&format!("pick({n}): stack underflow"));
            return;
        }
        let val = self.stack[len - 1 - n].clone();
        self.stack.push(val);
    }

    /// Pick with explicit index (used by tests and internal code).
    pub fn pick_n(&mut self, n: usize) {
        let len = self.stack.len();
        if n >= len {
            self.fault(&format!("pick({n}): stack underflow"));
            return;
        }
        let val = self.stack[len - 1 - n].clone();
        self.stack.push(val);
    }

    /// Copies the top item and inserts it before the second-to-top item.
    pub fn tuck(&mut self) {
        let len = self.stack.len();
        assert!(len >= 2, "stack underflow: tuck requires at least 2 items");
        let top = self.stack[len - 1].clone();
        self.stack.insert(len - 2, top);
    }

    /// Rotates the top three items: moves the third item to the top.
    pub fn rot(&mut self) {
        let len = self.stack.len();
        assert!(len >= 3, "stack underflow: rot requires at least 3 items");
        let val = self.stack.remove(len - 3);
        self.stack.push(val);
    }

    /// Pops an integer n from the stack, then moves the item at depth n to the top.
    pub fn roll(&mut self) {
        let n = self.pop_integer();
        #[allow(clippy::cast_sign_loss)]
        let n = n as usize;
        let len = self.stack.len();
        if n >= len {
            self.fault(&format!("roll({n}): stack underflow"));
            return;
        }
        let val = self.stack.remove(len - 1 - n);
        self.stack.push(val);
    }

    /// Reverses the top 3 items on the stack.
    pub fn reverse3(&mut self) {
        let len = self.stack.len();
        assert!(
            len >= 3,
            "stack underflow: reverse3 requires at least 3 items"
        );
        self.stack[len - 3..].reverse();
    }

    /// Reverses the top 4 items on the stack.
    pub fn reverse4(&mut self) {
        let len = self.stack.len();
        assert!(
            len >= 4,
            "stack underflow: reverse4 requires at least 4 items"
        );
        self.stack[len - 4..].reverse();
    }

    /// Pops n from the stack, then reverses the top n items.
    pub fn reverse_n(&mut self) {
        let n = self.pop_integer();
        #[allow(clippy::cast_sign_loss)]
        let n = n as usize;
        let len = self.stack.len();
        if n > len {
            self.fault(&format!("reverse_n({n}): stack underflow"));
            return;
        }
        if n > 1 {
            self.stack[len - n..].reverse();
        }
    }

    /// Pushes the number of items on the evaluation stack.
    pub fn depth(&mut self) {
        let d = self.stack.len() as i64;
        self.push_int(d);
    }

    /// Removes all items from the evaluation stack.
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    // ---------------------------------------------------------------
    // Variable access
    // ---------------------------------------------------------------

    /// Pushes the value of argument slot `index` onto the stack.
    pub fn load_arg(&mut self, index: usize) {
        let val = self.args[index].clone();
        self.stack.push(val);
    }

    /// Pops the top of the stack into argument slot `index`.
    pub fn store_arg(&mut self, index: usize) {
        let val = self.pop();
        self.args[index] = val;
    }

    /// Pushes the value of local slot `index` onto the stack.
    pub fn load_local(&mut self, index: usize) {
        let val = self.locals[index].clone();
        self.stack.push(val);
    }

    /// Pops the top of the stack into local slot `index`.
    pub fn store_local(&mut self, index: usize) {
        let val = self.pop();
        self.locals[index] = val;
    }

    /// Pushes the value of static field `index` onto the stack.
    pub fn load_static(&mut self, index: usize) {
        // Auto-extend static fields if needed (mirrors NeoVM behavior where
        // INITSSLOT may not have been called for this index yet).
        if index >= self.static_fields.len() {
            self.static_fields.resize(index + 1, StackValue::Null);
        }
        let val = self.static_fields[index].clone();
        self.stack.push(val);
    }

    /// Pops the top of the stack into static field `index`.
    pub fn store_static(&mut self, index: usize) {
        let val = self.pop();
        if index >= self.static_fields.len() {
            self.static_fields.resize(index + 1, StackValue::Null);
        }
        self.static_fields[index] = val;
    }

    // ---------------------------------------------------------------
    // Arithmetic (integer fast path)
    // ---------------------------------------------------------------

    /// Pops two integers and pushes their sum.
    pub fn add(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a.wrapping_add(b));
    }

    /// Pops two integers and pushes their difference (a - b).
    pub fn sub(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a.wrapping_sub(b));
    }

    /// Pops two integers and pushes their product.
    pub fn mul(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a.wrapping_mul(b));
    }

    /// Pops two values and pushes whether they are equal.
    pub fn equal(&mut self) {
        let b = self.pop();
        let a = self.pop();
        self.push_bool(a == b);
    }

    // ---------------------------------------------------------------
    // Exception handling
    // ---------------------------------------------------------------

    /// Throws an exception with the message from the top of the stack.
    /// If try frames exist, stores as pending error for `check_exception()` to handle.
    pub fn throw_ex(&mut self) {
        let val = self.pop();
        let msg = match &val {
            StackValue::ByteString(bytes) => {
                String::from_utf8_lossy(bytes).to_string()
            }
            StackValue::Integer(v) => format!("exception: {v}"),
            _ => format!("exception: {:?}", val),
        };
        if self.try_stack.iter().any(|f| !f.caught) {
            self.pending_error = Some(msg);
        } else {
            self.fault(&msg);
        }
    }

    /// Immediately faults the VM (unconditional abort).
    /// If try frames exist, stores as pending error.
    pub fn abort(&mut self) {
        if self.try_stack.iter().any(|f| !f.caught) {
            self.pending_error = Some("ABORT".to_string());
        } else {
            self.fault("ABORT");
        }
    }

    /// Faults the VM with a message from the top of the stack.
    /// If try frames exist, stores as pending error.
    pub fn abort_msg(&mut self) {
        let val = self.pop();
        let msg = match &val {
            StackValue::ByteString(bytes) => {
                format!("ABORTMSG: {}", String::from_utf8_lossy(bytes))
            }
            _ => format!("ABORTMSG: {:?}", val),
        };
        if self.try_stack.iter().any(|f| !f.caught) {
            self.pending_error = Some(msg);
        } else {
            self.fault(&msg);
        }
    }

    /// Pops a boolean; if `false`, faults the VM.
    /// If try frames exist, stores as pending error.
    pub fn assert_top(&mut self) {
        let val = self.pop();
        let is_true = match &val {
            StackValue::Boolean(b) => *b,
            StackValue::Integer(v) => *v != 0,
            _ => {
                self.fault("ASSERT: non-boolean/integer on stack");
                return;
            }
        };
        if !is_true {
            if self.try_stack.iter().any(|f| !f.caught) {
                self.pending_error = Some("ASSERT: assertion failed".to_string());
            } else {
                self.fault("ASSERT: assertion failed");
            }
        }
    }

    /// Pops a message, then pops a boolean; if `false`, faults with the message.
    /// If try frames exist, stores as pending error.
    pub fn assert_msg(&mut self) {
        let msg_val = self.pop();
        let cond_val = self.pop();
        let is_true = match &cond_val {
            StackValue::Boolean(b) => *b,
            StackValue::Integer(v) => *v != 0,
            _ => {
                self.fault("ASSERTMSG: non-boolean/integer condition");
                return;
            }
        };
        if !is_true {
            let msg = match &msg_val {
                StackValue::ByteString(bytes) => {
                    format!("ASSERTMSG: {}", String::from_utf8_lossy(bytes))
                }
                _ => format!("ASSERTMSG: {:?}", msg_val),
            };
            if self.try_stack.iter().any(|f| !f.caught) {
                self.pending_error = Some(msg);
            } else {
                self.fault(&msg);
            }
        }
    }

    /// Enters a try block with catch and finally PC offsets.
    /// Called by generated state machine code at TRY/TRY_L instructions.
    pub fn try_enter(&mut self, catch_pc: i32, finally_pc: i32) {
        self.try_stack.push(TryFrame {
            catch_pc,
            finally_pc,
            caught: false,
            in_finally: false,
            end_pc: 0,
        });
    }

    /// Ends a try/catch block. If the current frame has a finally block
    /// that hasn't run yet, returns the finally PC. Otherwise pops the
    /// frame and returns the target PC.
    /// Called by generated state machine code at ENDTRY/ENDTRY_L instructions.
    #[must_use]
    pub fn end_try(&mut self, target_pc: i32) -> i32 {
        if let Some(frame) = self.try_stack.last_mut() {
            if frame.finally_pc != 0 && !frame.in_finally {
                frame.end_pc = target_pc;
                frame.in_finally = true;
                return frame.finally_pc;
            }
        }
        self.try_stack.pop();
        target_pc
    }

    /// Ends a finally block. Returns the continuation PC, or -1 if a
    /// pending exception needs to be re-thrown to an outer handler.
    /// Called by generated state machine code at ENDFINALLY instructions.
    #[must_use]
    pub fn end_finally(&mut self) -> i32 {
        if let Some(frame) = self.try_stack.pop() {
            if frame.in_finally {
                if self.pending_error.is_some() {
                    // Re-throw: the next check_exception() call will handle it
                    return -1;
                }
                if frame.end_pc != 0 {
                    return frame.end_pc;
                }
            }
        }
        // No frame or not in finally — fault
        self.fault("ENDFINALLY without matching finally context");
        -1
    }

    /// Checks for a pending exception and handles it via the try stack.
    /// Returns Some(new_pc) if execution should jump to a catch or finally block.
    /// Returns None if there is no pending exception (or if unhandled — VM is faulted).
    /// Called at the top of each state machine loop iteration.
    #[must_use]
    pub fn check_exception(&mut self) -> Option<i32> {
        let error = self.pending_error.take()?;
        // Find the topmost uncaught frame
        let frame_idx = self.try_stack.iter().rposition(|f| !f.caught)?;
        let frame = &mut self.try_stack[frame_idx];
        frame.caught = true;
        let catch_pc = frame.catch_pc;
        let finally_pc = frame.finally_pc;
        if catch_pc != 0 {
            // Push the error message onto the stack for the catch block
            self.stack.push(StackValue::ByteString(error.into_bytes()));
            Some(catch_pc)
        } else if finally_pc != 0 {
            // No catch — go to finally, keep pending_error for re-throw
            self.try_stack[frame_idx].in_finally = true;
            self.pending_error = Some(error);
            Some(finally_pc)
        } else {
            // No catch or finally — fault
            self.fault(&error);
            None
        }
    }

    // ---------------------------------------------------------------
    // Interop / syscall
    // ---------------------------------------------------------------

    /// Invoke a host syscall by its interop hash.
    ///
    /// Delegates to the syscall bridge registered via `set_syscall_bridge`.
    /// If no bridge has been registered (e.g. in unit tests), this is a no-op.
    pub fn syscall(&mut self, hash: u32) {
        let bridge = unsafe { SYSCALL_BRIDGE };
        if let Some(f) = bridge {
            f(self, hash);
        }
        // If no bridge is set, silently do nothing (testing mode).
    }

    /// Stub: call a token-referenced method.
    ///
    /// Requires the host bridge for cross-contract invocation.
    pub fn call_token(&mut self, _token: u16) {
        self.fault("CALLT: not yet implemented (requires host bridge)");
    }

    /// Stub: call the address on top of the stack.
    pub fn calla(&mut self) {
        self.fault("CALLA: not yet implemented (requires host bridge)");
    }

    /// Converts the top stack value to the given NeoVM type.
    ///
    /// Delegates to `convert_to` in the conversion module.
    pub fn convert(&mut self, target_type: u8) {
        self.convert_to(target_type);
    }

    /// Push a return address onto the call stack (used by CALL).
    pub fn call_push(&mut self, return_pc: i32) {
        self.call_stack.push(return_pc);
    }

    /// Pop a return address from the call stack (used by RET).
    /// Returns `None` if the call stack is empty (entry-point method return),
    /// signaling the generated code should `return;` from the function.
    pub fn call_pop(&mut self) -> Option<i32> {
        self.call_stack.pop()
    }

    /// Alias for `call_pop` — the RET opcode pops the call stack and jumps
    /// back to the caller. Generated Rust code emits `ctx.ret()`.
    pub fn ret(&mut self) -> Option<i32> {
        self.call_pop()
    }

    // ---------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------

    /// Pops the top of the stack and extracts an `i64`.
    ///
    /// # Panics
    ///
    /// Panics if the top value is not an `Integer`.
    pub fn pop_integer(&mut self) -> i64 {
        match self.pop() {
            StackValue::Integer(v) => v,
            other => panic!(
                "expected Integer on stack, got tag {}",
                other.type_tag()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_abi_stack_roundtrip() {
        let abi_stack = vec![
            AbiStackValue::Integer(42),
            AbiStackValue::Boolean(true),
            AbiStackValue::Null,
        ];
        let ctx = Context::from_abi_stack(abi_stack);
        let result = ctx.to_execution_result(1000);
        assert_eq!(result.state, VmState::Halt);
        assert_eq!(result.fee_consumed_pico, 1000);
        assert_eq!(result.stack.len(), 3);
        assert_eq!(result.stack[0], AbiStackValue::Integer(42));
    }

    #[test]
    fn push_pop_operations() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(10);
        ctx.push_int(20);
        ctx.push_bool(false);
        ctx.push_bytes(b"hello");
        ctx.push_null();

        assert_eq!(ctx.stack.len(), 5);

        let val = ctx.pop();
        assert_eq!(val, StackValue::Null);

        ctx.drop();
        assert_eq!(ctx.stack.len(), 3);
    }

    #[test]
    fn dup_and_swap() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.dup();
        assert_eq!(ctx.stack.len(), 3);
        assert_eq!(ctx.pop(), StackValue::Integer(2));

        ctx.swap();
        assert_eq!(ctx.pop(), StackValue::Integer(1));
        assert_eq!(ctx.pop(), StackValue::Integer(2));
    }

    #[test]
    fn init_slot_pops_args() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(100);
        ctx.push_int(200);
        ctx.push_int(300);

        ctx.init_slot(2, 2);

        // 2 args popped: arg[0]=200, arg[1]=300 (reversed from pop order)
        assert_eq!(ctx.args.len(), 2);
        assert_eq!(ctx.args[0], StackValue::Integer(200));
        assert_eq!(ctx.args[1], StackValue::Integer(300));

        // 2 locals initialized to Null
        assert_eq!(ctx.locals.len(), 2);
        assert_eq!(ctx.locals[0], StackValue::Null);

        // stack still has the first push
        assert_eq!(ctx.stack.len(), 1);
        assert_eq!(ctx.pop(), StackValue::Integer(100));
    }

    #[test]
    fn load_store_locals() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.locals = vec![StackValue::Null; 3];

        ctx.push_int(42);
        ctx.store_local(1);
        ctx.load_local(1);

        assert_eq!(ctx.pop(), StackValue::Integer(42));
    }

    #[test]
    fn load_store_static() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(99);
        ctx.store_static(5);
        assert_eq!(ctx.static_fields.len(), 6);
        ctx.load_static(5);
        assert_eq!(ctx.pop(), StackValue::Integer(99));
    }

    #[test]
    fn arithmetic_ops() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(10);
        ctx.push_int(3);
        ctx.add();
        assert_eq!(ctx.pop(), StackValue::Integer(13));

        ctx.push_int(10);
        ctx.push_int(3);
        ctx.sub();
        assert_eq!(ctx.pop(), StackValue::Integer(7));

        ctx.push_int(6);
        ctx.push_int(7);
        ctx.mul();
        assert_eq!(ctx.pop(), StackValue::Integer(42));

        ctx.push_int(5);
        ctx.push_int(5);
        ctx.equal();
        assert_eq!(ctx.pop(), StackValue::Boolean(true));

        ctx.push_int(5);
        ctx.push_int(6);
        ctx.equal();
        assert_eq!(ctx.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn fault_sets_state() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.fault("something went wrong");
        assert_eq!(ctx.state, VmState::Fault);
        assert_eq!(
            ctx.fault_message.as_deref(),
            Some("something went wrong")
        );
    }

    // Stack operation tests

    #[test]
    fn nip_removes_second() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.nip();
        assert_eq!(ctx.stack.len(), 2);
        assert_eq!(ctx.pop(), StackValue::Integer(3));
        assert_eq!(ctx.pop(), StackValue::Integer(1));
    }

    #[test]
    fn xdrop_removes_nth() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(10);
        ctx.push_int(20);
        ctx.push_int(30);
        ctx.push_int(1); // index to drop
        ctx.xdrop();
        // Removed item at index 1 from top (which is 20)
        assert_eq!(ctx.stack.len(), 2);
        assert_eq!(ctx.pop(), StackValue::Integer(30));
        assert_eq!(ctx.pop(), StackValue::Integer(10));
    }

    #[test]
    fn over_copies_second() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.over();
        assert_eq!(ctx.stack.len(), 3);
        assert_eq!(ctx.pop(), StackValue::Integer(1));
    }

    #[test]
    fn pick_copies_nth() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(10);
        ctx.push_int(20);
        ctx.push_int(30);
        ctx.push_int(2);  // push index onto stack
        ctx.pick();
        assert_eq!(ctx.pop(), StackValue::Integer(10));
    }

    #[test]
    fn tuck_inserts_top_before_second() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.tuck();
        // Stack should be: 2, 1, 2
        assert_eq!(ctx.stack.len(), 3);
        assert_eq!(ctx.pop(), StackValue::Integer(2));
        assert_eq!(ctx.pop(), StackValue::Integer(1));
        assert_eq!(ctx.pop(), StackValue::Integer(2));
    }

    #[test]
    fn rot_moves_third_to_top() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.rot();
        assert_eq!(ctx.pop(), StackValue::Integer(1));
        assert_eq!(ctx.pop(), StackValue::Integer(3));
        assert_eq!(ctx.pop(), StackValue::Integer(2));
    }

    #[test]
    fn roll_moves_nth_to_top() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(10);
        ctx.push_int(20);
        ctx.push_int(30);
        ctx.push_int(2); // push index onto stack
        ctx.roll();
        assert_eq!(ctx.pop(), StackValue::Integer(10));
        assert_eq!(ctx.pop(), StackValue::Integer(30));
        assert_eq!(ctx.pop(), StackValue::Integer(20));
    }

    #[test]
    fn reverse3_and_reverse4() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.reverse3();
        assert_eq!(ctx.pop(), StackValue::Integer(1));
        assert_eq!(ctx.pop(), StackValue::Integer(2));
        assert_eq!(ctx.pop(), StackValue::Integer(3));

        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.push_int(4);
        ctx.reverse4();
        assert_eq!(ctx.pop(), StackValue::Integer(1));
        assert_eq!(ctx.pop(), StackValue::Integer(2));
        assert_eq!(ctx.pop(), StackValue::Integer(3));
        assert_eq!(ctx.pop(), StackValue::Integer(4));
    }

    #[test]
    fn reverse_n_op() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.push_int(4);
        ctx.push_int(5);
        ctx.push_int(3); // count to reverse
        ctx.reverse_n();
        assert_eq!(ctx.pop(), StackValue::Integer(3));
        assert_eq!(ctx.pop(), StackValue::Integer(4));
        assert_eq!(ctx.pop(), StackValue::Integer(5));
    }

    #[test]
    fn depth_and_clear() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_int(1);
        ctx.push_int(2);
        ctx.push_int(3);
        ctx.depth();
        assert_eq!(ctx.pop(), StackValue::Integer(3));

        ctx.clear();
        assert_eq!(ctx.stack.len(), 0);
    }

    // Exception handling tests

    #[test]
    fn throw_ex_faults() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push(StackValue::ByteString(b"test error".to_vec()));
        ctx.throw_ex();
        assert_eq!(ctx.state, VmState::Fault);
        assert_eq!(ctx.fault_message.as_deref(), Some("test error"));
    }

    #[test]
    fn abort_faults() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.abort();
        assert_eq!(ctx.state, VmState::Fault);
        assert_eq!(ctx.fault_message.as_deref(), Some("ABORT"));
    }

    #[test]
    fn abort_msg_faults() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push(StackValue::ByteString(b"custom error".to_vec()));
        ctx.abort_msg();
        assert_eq!(ctx.state, VmState::Fault);
        assert_eq!(
            ctx.fault_message.as_deref(),
            Some("ABORTMSG: custom error")
        );
    }

    #[test]
    fn assert_top_passes_on_true() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_bool(true);
        ctx.assert_top();
        assert_eq!(ctx.state, VmState::Halt);
    }

    #[test]
    fn assert_top_faults_on_false() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_bool(false);
        ctx.assert_top();
        assert_eq!(ctx.state, VmState::Fault);
    }

    #[test]
    fn assert_msg_faults_with_message() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_bool(false);
        ctx.push(StackValue::ByteString(b"check failed".to_vec()));
        ctx.assert_msg();
        assert_eq!(ctx.state, VmState::Fault);
        assert_eq!(
            ctx.fault_message.as_deref(),
            Some("ASSERTMSG: check failed")
        );
    }

    #[test]
    fn assert_msg_passes_on_true() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.push_bool(true);
        ctx.push(StackValue::ByteString(b"irrelevant".to_vec()));
        ctx.assert_msg();
        assert_eq!(ctx.state, VmState::Halt);
    }

    #[test]
    fn call_token_and_calla_fault() {
        let mut ctx = Context::from_abi_stack(vec![]);
        ctx.call_token(0);
        assert_eq!(ctx.state, VmState::Fault);

        ctx.state = VmState::Halt;
        ctx.fault_message = None;
        ctx.calla();
        assert_eq!(ctx.state, VmState::Fault);
    }

    #[test]
    fn is_faulted_reflects_state() {
        let mut ctx = Context::from_abi_stack(vec![]);
        assert!(!ctx.is_faulted());
        ctx.fault("boom");
        assert!(ctx.is_faulted());
    }
}
