#![allow(clippy::items_after_test_module)]

use crate::{pricing::charge_opcode, HostCallbackResult, RuntimeContext};
use neo_riscv_abi::{callback_codec, fast_codec};
use neo_riscv_guest::SyscallProvider;
use polkavm::Linker;
use std::collections::HashMap;
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
    linker
        .define_typed("host_storage_get", host_storage_get_import)
        .map_err(|e| e.to_string())?;
    linker
        .define_typed("host_storage_contains", host_storage_contains_import)
        .map_err(|e| e.to_string())?;
    linker
        .define_typed(
            "host_storage_contains_small32",
            host_storage_contains_small32_import,
        )
        .map_err(|e| e.to_string())?;
    linker
        .define_typed(
            "host_storage_put_and_contains",
            host_storage_put_and_contains_import,
        )
        .map_err(|e| e.to_string())?;
    linker
        .define_typed(
            "host_storage_put_and_contains_small32",
            host_storage_put_and_contains_small32_import,
        )
        .map_err(|e| e.to_string())?;
    linker
        .define_typed("host_storage_put", host_storage_put_import)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn host_on_instruction_import(caller: polkavm::Caller<ClosureHost>, opcode: u32) -> u32 {
    let host = caller.user_data;
    host.last_opcode = Some(opcode as u8);
    host.opcode_count = host.opcode_count.saturating_add(1);
    if let Err(e) = crate::pricing::check_instruction_ceiling(host.opcode_count) {
        host.charge_error = Some(e);
        return 0;
    }
    match charge_opcode(&mut host.context, &mut host.fee_consumed_pico, opcode as u8) {
        Ok(()) => 1,
        Err(e) => {
            host.charge_error = Some(e);
            0
        }
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

    if let Some(storage) = host.builtin_storage.as_mut() {
        if let Some(response) =
            try_handle_builtin_storage_syscall(api, &host.callback_read_buf, storage)
        {
            let bytes = match &response {
                BuiltinResponse::Static(bytes) => *bytes,
                BuiltinResponse::Owned(bytes) => bytes.as_slice(),
            };
            if bytes.len() > result_cap as usize {
                eprintln!(
                    "[neo-riscv-host] builtin host_call(api={api}): result buffer too small (need {} bytes, cap={result_cap})",
                    bytes.len()
                );
                return 0;
            }
            host.last_host_call_stage = 5;
            if let Err(e) = caller.instance.write_memory(result_ptr, bytes) {
                eprintln!(
                    "[neo-riscv-host] builtin host_call(api={api}): write_memory failed at ptr=0x{result_ptr:08x}: {e}"
                );
                return 0;
            }
            host.last_host_call_stage = 6;
            return bytes.len() as u32;
        }
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

fn host_storage_get_import(
    caller: polkavm::Caller<ClosureHost>,
    key_ptr: u32,
    key_len: u32,
    value_ptr: u32,
    value_cap: u32,
) -> u32 {
    let host = caller.user_data;
    let mut key_inline = [0u8; INLINE_KEY_CAP];
    let key_len_usize = key_len as usize;
    let value = if key_len_usize <= INLINE_KEY_CAP {
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_inline[..key_len_usize])
            .is_err()
        {
            return u32::MAX;
        }
        let Some(storage) = host.builtin_storage.as_mut() else {
            return u32::MAX;
        };
        let Some(value) = storage.get_promoting(&key_inline[..key_len_usize]) else {
            return u32::MAX;
        };
        value
    } else {
        let mut key_heap = vec![0u8; key_len_usize];
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_heap[..])
            .is_err()
        {
            return u32::MAX;
        }
        let Some(storage) = host.builtin_storage.as_mut() else {
            return u32::MAX;
        };
        let Some(value) = storage.get_promoting(key_heap.as_slice()) else {
            return u32::MAX;
        };
        value
    };

    if value.len() > value_cap as usize {
        return u32::MAX - 1;
    }

    if caller.instance.write_memory(value_ptr, value).is_err() {
        return u32::MAX;
    }

    value.len() as u32
}

fn host_storage_contains_import(
    caller: polkavm::Caller<ClosureHost>,
    key_ptr: u32,
    key_len: u32,
) -> u32 {
    let host = caller.user_data;
    let mut key_inline = [0u8; INLINE_KEY_CAP];
    let key_len_usize = key_len as usize;
    if key_len_usize <= INLINE_KEY_CAP {
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_inline[..key_len_usize])
            .is_err()
        {
            return 0;
        }
        let Some(storage) = host.builtin_storage.as_mut() else {
            return 0;
        };
        u32::from(
            storage
                .get_promoting(&key_inline[..key_len_usize])
                .is_some(),
        )
    } else {
        let mut key_heap = vec![0u8; key_len_usize];
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_heap[..])
            .is_err()
        {
            return 0;
        }
        let Some(storage) = host.builtin_storage.as_mut() else {
            return 0;
        };
        u32::from(storage.get_promoting(key_heap.as_slice()).is_some())
    }
}

fn host_storage_contains_small32_import(
    caller: polkavm::Caller<ClosureHost>,
    key_word: u32,
    key_len: u32,
) -> u32 {
    let host = caller.user_data;
    let Some(storage) = host.builtin_storage.as_mut() else {
        return 0;
    };

    let key_len = key_len as usize;
    if key_len > 4 {
        return 0;
    }

    let key_bytes = key_word.to_le_bytes();
    u32::from(storage.get_promoting(&key_bytes[..key_len]).is_some())
}

fn host_storage_put_import(
    caller: polkavm::Caller<ClosureHost>,
    key_ptr: u32,
    key_len: u32,
    value_ptr: u32,
    value_len: u32,
) -> u32 {
    let host = caller.user_data;
    let Some(storage) = host.builtin_storage.as_mut() else {
        return 0;
    };

    let key_len_usize = key_len as usize;
    let value_len_usize = value_len as usize;

    let mut key_inline = [0u8; INLINE_KEY_CAP];
    let mut value_inline = [0u8; INLINE_VALUE_CAP];

    if key_len_usize <= INLINE_KEY_CAP && value_len_usize <= INLINE_VALUE_CAP {
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_inline[..key_len_usize])
            .is_err()
            || caller
                .instance
                .read_memory_into(value_ptr, &mut value_inline[..value_len_usize])
                .is_err()
        {
            return 0;
        }

        storage.insert(
            &key_inline[..key_len_usize],
            &value_inline[..value_len_usize],
        );
        return 1;
    }

    let mut key_heap = vec![0u8; key_len_usize];
    let mut value_heap = vec![0u8; value_len_usize];
    if caller
        .instance
        .read_memory_into(key_ptr, &mut key_heap[..])
        .is_err()
        || caller
            .instance
            .read_memory_into(value_ptr, &mut value_heap[..])
            .is_err()
    {
        return 0;
    }

    storage.insert(&key_heap, &value_heap);
    1
}

fn host_storage_put_and_contains_import(
    caller: polkavm::Caller<ClosureHost>,
    key_ptr: u32,
    key_len: u32,
    value_ptr: u32,
    value_len: u32,
) -> u32 {
    let host = caller.user_data;
    let Some(storage) = host.builtin_storage.as_mut() else {
        return 0;
    };

    let key_len_usize = key_len as usize;
    let value_len_usize = value_len as usize;

    let mut key_inline = [0u8; INLINE_KEY_CAP];
    let mut value_inline = [0u8; INLINE_VALUE_CAP];

    if key_len_usize <= INLINE_KEY_CAP && value_len_usize <= INLINE_VALUE_CAP {
        if caller
            .instance
            .read_memory_into(key_ptr, &mut key_inline[..key_len_usize])
            .is_err()
            || caller
                .instance
                .read_memory_into(value_ptr, &mut value_inline[..value_len_usize])
                .is_err()
        {
            return 0;
        }

        storage.insert(
            &key_inline[..key_len_usize],
            &value_inline[..value_len_usize],
        );
        return 1;
    }

    let mut key_heap = vec![0u8; key_len_usize];
    let mut value_heap = vec![0u8; value_len_usize];
    if caller
        .instance
        .read_memory_into(key_ptr, &mut key_heap[..])
        .is_err()
        || caller
            .instance
            .read_memory_into(value_ptr, &mut value_heap[..])
            .is_err()
    {
        return 0;
    }

    storage.insert(&key_heap, &value_heap);
    1
}

fn host_storage_put_and_contains_small32_import(
    caller: polkavm::Caller<ClosureHost>,
    key_word: u32,
    key_len: u32,
    value_word: u32,
    value_len: u32,
) -> u32 {
    let host = caller.user_data;
    let Some(storage) = host.builtin_storage.as_mut() else {
        return 0;
    };

    let key_len = key_len as usize;
    let value_len = value_len as usize;
    if key_len > 4 || value_len > 4 {
        return 0;
    }

    let key_bytes = key_word.to_le_bytes();
    let value_bytes = value_word.to_le_bytes();
    u32::from(storage.put_small_inline(&key_bytes[..key_len], &value_bytes[..value_len]))
}

const ENCODED_OK_EMPTY: &[u8] = &[2];
const ENCODED_OK_NULL: &[u8] = &[5];
const ENCODED_OK_INT1: &[u8] = &[3, 1, 0, 0, 0, 0, 0, 0, 0];

enum BuiltinResponse {
    Static(&'static [u8]),
    Owned(Vec<u8>),
}

const INLINE_KEY_CAP: usize = 32;
const INLINE_VALUE_CAP: usize = 128;
const INLINE_ENTRY_SLOTS: usize = 8;
const SMALL_KEY_CAP: usize = 4;
const SMALL_VALUE_CAP: usize = 4;
const SMALL_ENTRY_SLOTS: usize = 8;

struct SmallStorageEntry {
    key_len: u8,
    key: [u8; SMALL_KEY_CAP],
    value_len: u8,
    value: [u8; SMALL_VALUE_CAP],
}

impl SmallStorageEntry {
    fn new(key: &[u8], value: &[u8]) -> Option<Self> {
        if key.len() > SMALL_KEY_CAP || value.len() > SMALL_VALUE_CAP {
            return None;
        }

        let mut key_buf = [0u8; SMALL_KEY_CAP];
        key_buf[..key.len()].copy_from_slice(key);
        let mut value_buf = [0u8; SMALL_VALUE_CAP];
        value_buf[..value.len()].copy_from_slice(value);

        Some(Self {
            key_len: key.len() as u8,
            key: key_buf,
            value_len: value.len() as u8,
            value: value_buf,
        })
    }

    fn key(&self) -> &[u8] {
        &self.key[..self.key_len as usize]
    }

    fn value(&self) -> &[u8] {
        &self.value[..self.value_len as usize]
    }

    fn matches(&self, key: &[u8]) -> bool {
        self.key() == key
    }

    fn overwrite_value(&mut self, value: &[u8]) -> bool {
        if value.len() > SMALL_VALUE_CAP {
            return false;
        }
        self.value[..value.len()].copy_from_slice(value);
        self.value_len = value.len() as u8;
        true
    }
}

struct InlineStorageEntry {
    key_len: u8,
    key: [u8; INLINE_KEY_CAP],
    value_len: u16,
    value: [u8; INLINE_VALUE_CAP],
}

impl InlineStorageEntry {
    fn new(key: &[u8], value: &[u8]) -> Option<Self> {
        if key.len() > INLINE_KEY_CAP || value.len() > INLINE_VALUE_CAP {
            return None;
        }

        let mut key_buf = [0u8; INLINE_KEY_CAP];
        key_buf[..key.len()].copy_from_slice(key);
        let mut value_buf = [0u8; INLINE_VALUE_CAP];
        value_buf[..value.len()].copy_from_slice(value);

        Some(Self {
            key_len: key.len() as u8,
            key: key_buf,
            value_len: value.len() as u16,
            value: value_buf,
        })
    }

    fn key(&self) -> &[u8] {
        &self.key[..self.key_len as usize]
    }

    fn value(&self) -> &[u8] {
        &self.value[..self.value_len as usize]
    }

    fn matches(&self, key: &[u8]) -> bool {
        self.key() == key
    }

    fn overwrite_value(&mut self, value: &[u8]) -> bool {
        if value.len() > INLINE_VALUE_CAP {
            return false;
        }
        self.value[..value.len()].copy_from_slice(value);
        self.value_len = value.len() as u16;
        true
    }
}

struct BuiltinStorage {
    hot_small: Option<SmallStorageEntry>,
    small: [Option<SmallStorageEntry>; SMALL_ENTRY_SLOTS],
    inline: [Option<InlineStorageEntry>; INLINE_ENTRY_SLOTS],
    heap: HashMap<Vec<u8>, Vec<u8>>,
}

impl BuiltinStorage {
    fn new() -> Self {
        Self {
            hot_small: None,
            small: std::array::from_fn(|_| None),
            inline: std::array::from_fn(|_| None),
            heap: HashMap::new(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        if let Some(entry) = &self.hot_small {
            if entry.matches(key) {
                return Some(entry.value());
            }
        }
        for entry in self.small.iter().flatten() {
            if entry.matches(key) {
                return Some(entry.value());
            }
        }
        for entry in self.inline.iter().flatten() {
            if entry.matches(key) {
                return Some(entry.value());
            }
        }
        self.heap.get(key).map(Vec::as_slice)
    }

    fn get_promoting(&mut self, key: &[u8]) -> Option<&[u8]> {
        if self
            .hot_small
            .as_ref()
            .is_some_and(|entry| entry.matches(key))
        {
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        if let Some(index) = self.find_small_index(key) {
            let promoted = {
                let entry = self.small[index]
                    .as_ref()
                    .expect("small slot index must reference an entry");
                SmallStorageEntry::new(entry.key(), entry.value())
            };
            self.hot_small = promoted;
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        if let Some(index) = self.find_inline_index(key) {
            let promoted = {
                let entry = self.inline[index]
                    .as_ref()
                    .expect("inline slot index must reference an entry");
                SmallStorageEntry::new(entry.key(), entry.value())
            };
            if let Some(entry) = promoted {
                self.hot_small = Some(entry);
                return self.hot_small.as_ref().map(|entry| entry.value());
            }
            return self.inline[index].as_ref().map(|entry| entry.value());
        }

        if let Some(entry) = self
            .heap
            .get(key)
            .and_then(|value| SmallStorageEntry::new(key, value))
        {
            self.hot_small = Some(entry);
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        self.heap.get(key).map(Vec::as_slice)
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.refresh_hot_small(key, value);

        if let Some(index) = self.find_small_index(key) {
            if self.small[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return;
            }
            self.small[index] = None;
        }

        if let Some(entry) = SmallStorageEntry::new(key, value) {
            if let Some(slot) = self.small.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(entry);
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return;
            }
        }

        if let Some(index) = self.find_inline_index(key) {
            if self.inline[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.heap.remove(key);
                return;
            }
            self.inline[index] = None;
        }

        if let Some(entry) = InlineStorageEntry::new(key, value) {
            if let Some(slot) = self.inline.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(entry);
                self.heap.remove(key);
                return;
            }
        }

        self.heap.insert(key.to_vec(), value.to_vec());
    }

    fn put_small_inline(&mut self, key: &[u8], value: &[u8]) -> bool {
        self.refresh_hot_small(key, value);

        if let Some(index) = self.find_small_index(key) {
            if self.small[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return true;
            }
            self.small[index] = None;
        }

        if let Some(entry) = SmallStorageEntry::new(key, value) {
            if let Some(slot) = self.small.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(entry);
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return true;
            }
        }

        if let Some(index) = self.find_inline_index(key) {
            if self.inline[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.heap.remove(key);
                return true;
            }
            self.inline[index] = None;
        }

        if let Some(entry) = InlineStorageEntry::new(key, value) {
            if let Some(slot) = self.inline.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(entry);
                self.heap.remove(key);
                return true;
            }
        }
        false
    }

    fn find_small_index(&self, key: &[u8]) -> Option<usize> {
        self.small
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|entry| entry.matches(key)))
    }

    fn find_inline_index(&self, key: &[u8]) -> Option<usize> {
        self.inline
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|entry| entry.matches(key)))
    }

    fn refresh_hot_small(&mut self, key: &[u8], value: &[u8]) {
        match SmallStorageEntry::new(key, value) {
            Some(entry) => self.hot_small = Some(entry),
            None if self
                .hot_small
                .as_ref()
                .is_some_and(|entry| entry.matches(key)) =>
            {
                self.hot_small = None;
            }
            None => {}
        }
    }

    fn remove(&mut self, key: &[u8]) {
        if self
            .hot_small
            .as_ref()
            .is_some_and(|entry| entry.matches(key))
        {
            self.hot_small = None;
        }
        for slot in &mut self.small {
            if slot.as_ref().is_some_and(|entry| entry.matches(key)) {
                *slot = None;
                return;
            }
        }
        self.remove_from_inline_only(key);
        self.heap.remove(key);
    }

    fn remove_from_inline_only(&mut self, key: &[u8]) {
        for slot in &mut self.inline {
            if slot.as_ref().is_some_and(|entry| entry.matches(key)) {
                *slot = None;
                return;
            }
        }
    }
}

fn try_handle_builtin_storage_syscall(
    api: u32,
    raw_stack: &[u8],
    storage: &mut BuiltinStorage,
) -> Option<BuiltinResponse> {
    use neo_riscv_abi::interop_hash;
    use neo_riscv_abi::StackValue;

    if api == interop_hash("System.Storage.GetContext")
        || api == interop_hash("System.Storage.GetReadOnlyContext")
        || api == interop_hash("System.Storage.AsReadOnly")
    {
        return Some(BuiltinResponse::Static(ENCODED_OK_INT1));
    }

    if api == interop_hash("System.Storage.Get") || api == interop_hash("System.Storage.Local.Get")
    {
        let key = extract_last_bytestring(raw_stack)?;

        return Some(match storage.get_promoting(key) {
            Some(value) => BuiltinResponse::Owned(callback_codec::encode_stack_result(&Ok(vec![
                StackValue::ByteString(value.to_vec()),
            ]))),
            None => BuiltinResponse::Static(ENCODED_OK_NULL),
        });
    }

    if api == interop_hash("System.Storage.Put") || api == interop_hash("System.Storage.Local.Put")
    {
        let (key, value) = extract_last_two_bytestrings(raw_stack)?;
        storage.insert(&key, &value);
        return Some(BuiltinResponse::Static(ENCODED_OK_EMPTY));
    }

    if api == interop_hash("System.Storage.Delete")
        || api == interop_hash("System.Storage.Local.Delete")
    {
        if let Some(key) = extract_last_bytestring(raw_stack) {
            storage.remove(key);
        }
        return Some(BuiltinResponse::Static(ENCODED_OK_EMPTY));
    }

    if api == interop_hash("System.Storage.Find")
        || api == interop_hash("System.Storage.Local.Find")
    {
        return Some(BuiltinResponse::Static(ENCODED_OK_NULL));
    }

    None
}

fn extract_last_bytestring(raw_stack: &[u8]) -> Option<&[u8]> {
    let count = read_u32(raw_stack, 0)? as usize;
    let mut offset = 4;
    let mut last = None;

    for _ in 0..count {
        let (tag, next) = read_u8(raw_stack, offset)?;
        offset = next;
        match tag {
            0x03 | 0x02 | 0x0C => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                let end = offset.checked_add(len)?;
                if end > raw_stack.len() {
                    return None;
                }
                last = Some(&raw_stack[offset..end]);
                offset = end;
            }
            0x01 | 0x0B | 0x08 | 0x09 => offset += 8,
            0x04 => offset += 1,
            0x0A => {}
            0x05 | 0x06 => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                for _ in 0..len {
                    offset = skip_value(raw_stack, offset)?;
                }
            }
            0x07 => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                for _ in 0..len {
                    offset = skip_value(raw_stack, offset)?;
                    offset = skip_value(raw_stack, offset)?;
                }
            }
            _ => return None,
        }
    }

    last
}

fn extract_last_two_bytestrings(raw_stack: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let count = read_u32(raw_stack, 0)? as usize;
    let mut offset = 4;
    let mut found: Vec<Vec<u8>> = Vec::with_capacity(2);

    for _ in 0..count {
        let (tag, next) = read_u8(raw_stack, offset)?;
        offset = next;
        match tag {
            0x03 | 0x02 | 0x0C => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                let end = offset.checked_add(len)?;
                if end > raw_stack.len() {
                    return None;
                }
                found.push(raw_stack[offset..end].to_vec());
                if found.len() > 2 {
                    found.remove(0);
                }
                offset = end;
            }
            0x01 | 0x0B | 0x08 | 0x09 => offset += 8,
            0x04 => offset += 1,
            0x0A => {}
            0x05 | 0x06 => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                for _ in 0..len {
                    offset = skip_value(raw_stack, offset)?;
                }
            }
            0x07 => {
                let len = read_u32(raw_stack, offset)? as usize;
                offset += 4;
                for _ in 0..len {
                    offset = skip_value(raw_stack, offset)?;
                    offset = skip_value(raw_stack, offset)?;
                }
            }
            _ => return None,
        }
    }

    if found.len() == 2 {
        Some((found[0].clone(), found[1].clone()))
    } else {
        None
    }
}

fn skip_value(raw_stack: &[u8], offset: usize) -> Option<usize> {
    let (tag, mut next) = read_u8(raw_stack, offset)?;
    match tag {
        0x01 | 0x0B | 0x08 | 0x09 => Some(next + 8),
        0x03 | 0x02 | 0x0C => {
            let len = read_u32(raw_stack, next)? as usize;
            next += 4;
            let end = next.checked_add(len)?;
            (end <= raw_stack.len()).then_some(end)
        }
        0x04 => Some(next + 1),
        0x0A => Some(next),
        0x05 | 0x06 => {
            let len = read_u32(raw_stack, next)? as usize;
            next += 4;
            for _ in 0..len {
                next = skip_value(raw_stack, next)?;
            }
            Some(next)
        }
        0x07 => {
            let len = read_u32(raw_stack, next)? as usize;
            next += 4;
            for _ in 0..len {
                next = skip_value(raw_stack, next)?;
                next = skip_value(raw_stack, next)?;
            }
            Some(next)
        }
        _ => None,
    }
}

fn read_u8(raw: &[u8], offset: usize) -> Option<(u8, usize)> {
    raw.get(offset).copied().map(|b| (b, offset + 1))
}

fn read_u32(raw: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = raw.get(offset..end)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::{BuiltinStorage, INLINE_VALUE_CAP, SMALL_ENTRY_SLOTS};

    #[test]
    fn small_entry_round_trip_stays_in_small_cache() {
        let mut storage = BuiltinStorage::new();
        storage.insert(&[0x01, 0x02], &[0x0a, 0x0b, 0x0c]);
        assert_eq!(storage.get(&[0x01, 0x02]), Some(&[0x0a, 0x0b, 0x0c][..]));

        storage.insert(&[0x01, 0x02], &[0x7f]);
        assert_eq!(storage.get(&[0x01, 0x02]), Some(&[0x7f][..]));
    }

    #[test]
    fn small_inline_slots_can_hold_multiple_entries() {
        let mut storage = BuiltinStorage::new();
        for i in 0u8..6 {
            storage.insert(&[i], &[i + 10]);
        }

        for i in 0u8..6 {
            assert_eq!(storage.get(&[i]), Some(&[i + 10][..]));
        }
    }

    #[test]
    fn large_entry_falls_back_without_breaking_small_entries() {
        let mut storage = BuiltinStorage::new();
        storage.insert(&[0x01], &[0x11]);
        storage.insert(&[0x02; 40], &[0x03; 140]);

        assert_eq!(storage.get(&[0x01]), Some(&[0x11][..]));
        assert_eq!(storage.get(&[0x02; 40]), Some(&[0x03; 140][..]));
    }

    #[test]
    fn remove_clears_small_and_heap_entries() {
        let mut storage = BuiltinStorage::new();
        storage.insert(&[0x01], &[0x10]);
        storage.insert(&[0x02; 40], &[0x20; 140]);

        storage.remove(&[0x01]);
        storage.remove(&[0x02; 40]);

        assert_eq!(storage.get(&[0x01]), None);
        assert_eq!(storage.get(&[0x02; 40]), None);
    }

    #[test]
    fn get_promoting_rehydrates_hot_small_from_inline_entries() {
        let mut storage = BuiltinStorage::new();
        for i in 0u8..SMALL_ENTRY_SLOTS as u8 {
            storage.insert(&[i], &[i + 1]);
        }

        storage.insert(&[0x55], &[0x66]);
        storage.insert(&[0x77], &[0x88]);
        storage.hot_small = None;

        assert_eq!(storage.get_promoting(&[0x55]), Some(&[0x66][..]));
        assert_eq!(
            storage.hot_small.as_ref().map(|entry| entry.key()),
            Some(&[0x55][..])
        );
        assert_eq!(
            storage.hot_small.as_ref().map(|entry| entry.value()),
            Some(&[0x66][..])
        );
    }

    #[test]
    fn insert_migrates_small_entries_without_returning_stale_values() {
        let mut storage = BuiltinStorage::new();
        let key = [0x01];
        let large_value = [0xAB; 12];

        storage.insert(&key, &[0x11]);
        storage.insert(&key, &large_value);

        assert_eq!(storage.get(&key), Some(large_value.as_slice()));
        assert!(
            storage.hot_small.is_none(),
            "non-small updates must evict stale hot_small entries"
        );
    }

    #[test]
    fn insert_migrates_inline_entries_to_heap_without_returning_stale_values() {
        let mut storage = BuiltinStorage::new();
        for i in 0u8..SMALL_ENTRY_SLOTS as u8 {
            storage.insert(&[i], &[i + 1]);
        }

        let key = [0x55];
        let inline_value = [0xBC; 12];
        let heap_value = [0xCD; INLINE_VALUE_CAP + 8];

        storage.insert(&key, &inline_value);
        storage.insert(&key, &heap_value);

        assert_eq!(storage.get(&key), Some(heap_value.as_slice()));
        assert!(
            storage
                .hot_small
                .as_ref()
                .is_none_or(|entry| !entry.matches(&key)),
            "large updates must not leave stale hot_small entries for the migrated key"
        );
    }
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
    // Populated by `host_on_instruction_import` / `ClosureHost::on_instruction` when the
    // host-side charge returns Err (gas exhaustion or instruction-count ceiling). The guest
    // only sees the import's 0-return and propagates a generic
    // "host instruction charge failed" — this field lets `execute_script_with_host_and_stack_and_ip`
    // recover the specific reason and surface it to C#.
    pub(crate) charge_error: Option<String>,
    builtin_storage: Option<BuiltinStorage>,
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
            charge_error: None,
            builtin_storage: None,
            callback_data: callback as *mut F as *mut c_void,
            callback_invoke: invoke_callback::<F>,
        }
    }

    pub(crate) fn new_builtin(context: RuntimeContext) -> Self {
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
            charge_error: None,
            builtin_storage: Some(BuiltinStorage::new()),
            callback_data: std::ptr::null_mut(),
            callback_invoke: invoke_builtin_fallback,
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

unsafe fn invoke_builtin_fallback(
    _callback_data: *mut c_void,
    api: u32,
    _ip: usize,
    context: RuntimeContext,
    stack: &[neo_riscv_abi::StackValue],
) -> HostCallbackOutcome {
    crate::builtin_host_callback(api, context, stack)
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
        if let Err(e) = crate::pricing::check_instruction_ceiling(self.opcode_count) {
            self.charge_error = Some(e.clone());
            return Err(e);
        }
        match charge_opcode(&mut self.context, &mut self.fee_consumed_pico, opcode) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.charge_error = Some(e.clone());
                Err(e)
            }
        }
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
