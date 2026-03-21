extern crate alloc;

use alloc::vec::Vec;
use neo_riscv_abi::StackValue as AbiStackValue;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StackValue {
    Integer(i64),
    BigInteger(Vec<u8>),
    ByteString(Vec<u8>),
    Boolean(bool),
    Pointer(usize),
    Array(u64, Vec<StackValue>),
    Struct(u64, Vec<StackValue>),
    Map(u64, Vec<(StackValue, StackValue)>),
    Buffer(u64, Vec<u8>),
    Interop(u64),
    Iterator(u64),
    Null,
}

#[derive(Default)]
pub(crate) struct CompoundIds {
    next: u64,
}

impl CompoundIds {
    fn alloc(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    pub(crate) fn array(&mut self, items: Vec<StackValue>) -> StackValue {
        StackValue::Array(self.alloc(), items)
    }

    pub(crate) fn r#struct(&mut self, items: Vec<StackValue>) -> StackValue {
        StackValue::Struct(self.alloc(), items)
    }

    pub(crate) fn map(&mut self, items: Vec<(StackValue, StackValue)>) -> StackValue {
        StackValue::Map(self.alloc(), items)
    }

    pub(crate) fn buffer(&mut self, bytes: Vec<u8>) -> StackValue {
        StackValue::Buffer(self.alloc(), bytes)
    }

    pub(crate) fn clone_struct_for_storage(&mut self, value: &StackValue) -> StackValue {
        match value {
            StackValue::Struct(_, _) => self.deep_clone(value),
            _ => value.clone(),
        }
    }

    pub(crate) fn deep_clone(&mut self, value: &StackValue) -> StackValue {
        match value {
            StackValue::Integer(value) => StackValue::Integer(*value),
            StackValue::BigInteger(value) => StackValue::BigInteger(value.clone()),
            StackValue::ByteString(value) => StackValue::ByteString(value.clone()),
            StackValue::Boolean(value) => StackValue::Boolean(*value),
            StackValue::Pointer(value) => StackValue::Pointer(*value),
            StackValue::Array(_, items) => {
                let cloned = items.iter().map(|item| self.deep_clone(item)).collect();
                self.array(cloned)
            }
            StackValue::Struct(_, items) => {
                let cloned = items.iter().map(|item| self.deep_clone(item)).collect();
                self.r#struct(cloned)
            }
            StackValue::Map(_, items) => {
                let cloned = items
                    .iter()
                    .map(|(key, value)| (self.deep_clone(key), self.deep_clone(value)))
                    .collect();
                self.map(cloned)
            }
            StackValue::Buffer(_, bytes) => self.buffer(bytes.clone()),
            StackValue::Interop(handle) => StackValue::Interop(*handle),
            StackValue::Iterator(handle) => StackValue::Iterator(*handle),
            StackValue::Null => StackValue::Null,
        }
    }

    pub(crate) fn import_abi(&mut self, value: AbiStackValue) -> StackValue {
        match value {
            AbiStackValue::Integer(value) => StackValue::Integer(value),
            AbiStackValue::BigInteger(value) => StackValue::BigInteger(value),
            AbiStackValue::ByteString(value) => StackValue::ByteString(value),
            AbiStackValue::Boolean(value) => StackValue::Boolean(value),
            AbiStackValue::Pointer(value) => StackValue::Pointer(value as usize),
            AbiStackValue::Array(items) => {
                let imported = items
                    .into_iter()
                    .map(|item| self.import_abi(item))
                    .collect();
                self.array(imported)
            }
            AbiStackValue::Struct(items) => {
                let imported = items
                    .into_iter()
                    .map(|item| self.import_abi(item))
                    .collect();
                self.r#struct(imported)
            }
            AbiStackValue::Map(items) => {
                let imported = items
                    .into_iter()
                    .map(|(key, value)| (self.import_abi(key), self.import_abi(value)))
                    .collect();
                self.map(imported)
            }
            AbiStackValue::Interop(handle) => StackValue::Interop(handle),
            AbiStackValue::Iterator(handle) => StackValue::Iterator(handle),
            AbiStackValue::Null => StackValue::Null,
        }
    }
}

pub(crate) fn to_abi_stack(stack: &[StackValue]) -> Vec<AbiStackValue> {
    let mut abi = Vec::with_capacity(stack.len().max(8));
    for item in stack {
        abi.push(to_abi_value(item));
    }
    abi
}

pub(crate) fn to_abi_value(value: &StackValue) -> AbiStackValue {
    match value {
        StackValue::Integer(value) => AbiStackValue::Integer(*value),
        StackValue::BigInteger(value) => AbiStackValue::BigInteger(clone_bytes(value)),
        StackValue::ByteString(value) => AbiStackValue::ByteString(clone_bytes(value)),
        StackValue::Boolean(value) => AbiStackValue::Boolean(*value),
        StackValue::Pointer(value) => AbiStackValue::Pointer(*value as i64),
        StackValue::Array(_, items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(to_abi_value(item));
            }
            AbiStackValue::Array(converted)
        }
        StackValue::Struct(_, items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(to_abi_value(item));
            }
            AbiStackValue::Struct(converted)
        }
        StackValue::Map(_, items) => {
            let mut converted = Vec::with_capacity(items.len());
            for (key, value) in items {
                converted.push((to_abi_value(key), to_abi_value(value)));
            }
            AbiStackValue::Map(converted)
        }
        StackValue::Buffer(_, bytes) => AbiStackValue::ByteString(clone_bytes(bytes)),
        StackValue::Interop(handle) => AbiStackValue::Interop(*handle),
        StackValue::Iterator(handle) => AbiStackValue::Iterator(*handle),
        StackValue::Null => AbiStackValue::Null,
    }
}

fn clone_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(bytes);
    out
}

fn compound_id(value: &StackValue) -> Option<u64> {
    match value {
        StackValue::Array(id, _)
        | StackValue::Struct(id, _)
        | StackValue::Map(id, _)
        | StackValue::Buffer(id, _) => Some(*id),
        _ => None,
    }
}

pub(crate) fn propagate_update(
    updated: &StackValue,
    stack: &mut [StackValue],
    locals: &mut [StackValue],
    static_fields: &mut [StackValue],
) {
    for value in stack {
        replace_alias(value, updated);
    }
    for value in locals {
        replace_alias(value, updated);
    }
    for value in static_fields {
        replace_alias(value, updated);
    }
}

fn replace_alias(target: &mut StackValue, updated: &StackValue) {
    if compound_id(target).is_some() && compound_id(target) == compound_id(updated) {
        *target = updated.clone();
        return;
    }

    match target {
        StackValue::Array(_, items) | StackValue::Struct(_, items) => {
            for item in items {
                replace_alias(item, updated);
            }
        }
        StackValue::Map(_, items) => {
            for (key, value) in items {
                replace_alias(key, updated);
                replace_alias(value, updated);
            }
        }
        StackValue::Buffer(_, _)
        | StackValue::Integer(_)
        | StackValue::BigInteger(_)
        | StackValue::ByteString(_)
        | StackValue::Boolean(_)
        | StackValue::Pointer(_)
        | StackValue::Interop(_)
        | StackValue::Iterator(_)
        | StackValue::Null => {}
    }
}
