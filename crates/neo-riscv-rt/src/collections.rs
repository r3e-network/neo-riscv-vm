//! Collection operations for the NeoVM `Context`.
//!
//! Implements array, struct, map, and buffer creation and manipulation.

use crate::stack_value::{StackValue, TAG_ARRAY, TAG_BUFFER, TAG_BYTESTRING, TAG_MAP, TAG_STRUCT};
use crate::Context;
use alloc::vec;
use alloc::vec::Vec;

impl Context {
    // ---------------------------------------------------------------
    // Collection constructors
    // ---------------------------------------------------------------

    /// Pushes an empty array onto the stack.
    pub fn new_array_0(&mut self) {
        self.push(StackValue::Array(Vec::new()));
    }

    /// Pops a count and pushes an array of that many `Null` values.
    pub fn new_array(&mut self) {
        let count = self.pop_integer();
        if count < 0 {
            self.fault("NEWARRAY: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let arr = vec![StackValue::Null; count as usize];
        self.push(StackValue::Array(arr));
    }

    /// Pops a count and pushes a typed array (for now, all items are default for type).
    pub fn new_array_t(&mut self, type_byte: u8) {
        let count = self.pop_integer();
        if count < 0 {
            self.fault("NEWARRAY_T: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let default_val = default_for_type(type_byte);
        let arr = vec![default_val; count as usize];
        self.push(StackValue::Array(arr));
    }

    /// Pushes an empty struct onto the stack.
    pub fn new_struct_0(&mut self) {
        self.push(StackValue::Struct(Vec::new()));
    }

    /// Pops a count and pushes a struct of that many `Null` values.
    pub fn new_struct(&mut self) {
        let count = self.pop_integer();
        if count < 0 {
            self.fault("NEWSTRUCT: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let s = vec![StackValue::Null; count as usize];
        self.push(StackValue::Struct(s));
    }

    /// Pushes an empty map onto the stack.
    pub fn new_map(&mut self) {
        self.push(StackValue::Map(Vec::new()));
    }

    /// Pops a size and pushes a zero-filled buffer of that size.
    pub fn new_buffer(&mut self) {
        let size = self.pop_integer();
        if size < 0 {
            self.fault("NEWBUFFER: negative size");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        self.push(StackValue::Buffer(vec![0u8; size as usize]));
    }

    // ---------------------------------------------------------------
    // Collection operations
    // ---------------------------------------------------------------

    /// Pops a value then an array/struct/map and appends the value.
    pub fn append(&mut self) {
        let value = self.pop();
        let collection = self.stack.last_mut();
        match collection {
            Some(StackValue::Array(ref mut items)) => {
                items.push(value);
            }
            Some(StackValue::Struct(ref mut items)) => {
                items.push(value);
            }
            _ => {
                self.fault("APPEND: top-1 is not an array or struct");
            }
        }
    }

    /// Pops value, key, then collection and sets collection[key] = value.
    pub fn set_item(&mut self) {
        let value = self.pop();
        let key = self.pop();
        let collection = self.stack.last_mut();
        match collection {
            Some(StackValue::Array(ref mut items)) | Some(StackValue::Struct(ref mut items)) => {
                if let StackValue::Integer(idx) = key {
                    #[allow(clippy::cast_sign_loss)]
                    let idx = idx as usize;
                    if idx >= items.len() {
                        self.fault("SETITEM: index out of range");
                        return;
                    }
                    items[idx] = value;
                } else {
                    self.fault("SETITEM: non-integer index for array/struct");
                }
            }
            Some(StackValue::Map(ref mut pairs)) => {
                // Update existing or insert new.
                for pair in pairs.iter_mut() {
                    if pair.0 == key {
                        pair.1 = value;
                        return;
                    }
                }
                pairs.push((key, value));
            }
            Some(StackValue::Buffer(ref mut buf)) => {
                if let (StackValue::Integer(idx), StackValue::Integer(val)) = (&key, &value) {
                    #[allow(clippy::cast_sign_loss)]
                    let idx = *idx as usize;
                    if idx >= buf.len() {
                        self.fault("SETITEM: buffer index out of range");
                        return;
                    }
                    #[allow(clippy::cast_sign_loss)]
                    {
                        buf[idx] = *val as u8;
                    }
                } else {
                    self.fault("SETITEM: buffer requires integer key and value");
                }
            }
            _ => {
                self.fault("SETITEM: not a collection");
            }
        }
    }

    /// Pops key then collection and pushes collection[key].
    pub fn pick_item(&mut self) {
        let key = self.pop();
        let collection = self.pop();
        match (&collection, &key) {
            (StackValue::Array(items) | StackValue::Struct(items), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                if idx >= items.len() {
                    self.fault("PICKITEM: index out of range");
                    return;
                }
                self.push(items[idx].clone());
            }
            (StackValue::Map(pairs), _) => {
                for (k, v) in pairs {
                    if *k == key {
                        self.push(v.clone());
                        return;
                    }
                }
                self.fault("PICKITEM: key not found in map");
            }
            (StackValue::ByteString(bytes), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                if idx >= bytes.len() {
                    self.fault("PICKITEM: byte index out of range");
                    return;
                }
                self.push_int(i64::from(bytes[idx]));
            }
            (StackValue::Buffer(buf), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                if idx >= buf.len() {
                    self.fault("PICKITEM: buffer index out of range");
                    return;
                }
                self.push_int(i64::from(buf[idx]));
            }
            _ => {
                self.fault("PICKITEM: unsupported types");
            }
        }
    }

    /// Pops key then collection and removes the entry.
    pub fn remove(&mut self) {
        let key = self.pop();
        let collection = self.stack.last_mut();
        match collection {
            Some(StackValue::Array(ref mut items)) | Some(StackValue::Struct(ref mut items)) => {
                if let StackValue::Integer(idx) = key {
                    #[allow(clippy::cast_sign_loss)]
                    let idx = idx as usize;
                    if idx >= items.len() {
                        self.fault("REMOVE: index out of range");
                        return;
                    }
                    items.remove(idx);
                } else {
                    self.fault("REMOVE: non-integer index for array/struct");
                }
            }
            Some(StackValue::Map(ref mut pairs)) => {
                pairs.retain(|(k, _)| *k != key);
            }
            _ => {
                self.fault("REMOVE: not a collection");
            }
        }
    }

    /// Pops a collection/string/buffer and pushes its size.
    pub fn size(&mut self) {
        let val = self.pop();
        match &val {
            StackValue::Array(items) | StackValue::Struct(items) => {
                self.push_int(items.len() as i64);
            }
            StackValue::Map(pairs) => {
                self.push_int(pairs.len() as i64);
            }
            StackValue::ByteString(bytes) => {
                self.push_int(bytes.len() as i64);
            }
            StackValue::Buffer(buf) => {
                self.push_int(buf.len() as i64);
            }
            _ => {
                self.fault("SIZE: unsupported type");
            }
        }
    }

    /// Pops key then collection and pushes `true` if the key exists.
    pub fn has_key(&mut self) {
        let key = self.pop();
        let collection = self.pop();
        match (&collection, &key) {
            (StackValue::Array(items) | StackValue::Struct(items), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                self.push_bool(idx < items.len());
            }
            (StackValue::Map(pairs), _) => {
                let found = pairs.iter().any(|(k, _)| *k == key);
                self.push_bool(found);
            }
            (StackValue::ByteString(bytes), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                self.push_bool(idx < bytes.len());
            }
            (StackValue::Buffer(buf), StackValue::Integer(idx)) => {
                #[allow(clippy::cast_sign_loss)]
                let idx = *idx as usize;
                self.push_bool(idx < buf.len());
            }
            _ => {
                self.fault("HASKEY: unsupported types");
            }
        }
    }

    /// Pops a map and pushes an array of its keys.
    pub fn keys(&mut self) {
        let val = self.pop();
        match val {
            StackValue::Map(pairs) => {
                let key_arr: Vec<StackValue> = pairs.into_iter().map(|(k, _)| k).collect();
                self.push(StackValue::Array(key_arr));
            }
            _ => {
                self.fault("KEYS: not a map");
            }
        }
    }

    /// Pops a map and pushes an array of its values.
    pub fn values(&mut self) {
        let val = self.pop();
        match val {
            StackValue::Map(pairs) => {
                let val_arr: Vec<StackValue> = pairs.into_iter().map(|(_, v)| v).collect();
                self.push(StackValue::Array(val_arr));
            }
            StackValue::Array(items) | StackValue::Struct(items) => {
                // NeoVM VALUES on array returns the array as-is (as a new array).
                self.push(StackValue::Array(items));
            }
            _ => {
                self.fault("VALUES: not a map or array");
            }
        }
    }

    /// Pops count, then that many items, and pushes them as an array.
    pub fn pack(&mut self) {
        let count = self.pop_integer();
        if count < 0 {
            self.fault("PACK: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(self.pop());
        }
        items.reverse();
        self.push(StackValue::Array(items));
    }

    /// Pops an array and pushes all its items then the count.
    pub fn unpack(&mut self) {
        let val = self.pop();
        match val {
            StackValue::Array(items) | StackValue::Struct(items) => {
                let count = items.len() as i64;
                for item in items {
                    self.push(item);
                }
                self.push_int(count);
            }
            _ => {
                self.fault("UNPACK: not an array or struct");
            }
        }
    }

    /// Pops a collection and pushes it with items in reverse order.
    pub fn reverse_items(&mut self) {
        let collection = self.stack.last_mut();
        match collection {
            Some(StackValue::Array(ref mut items)) | Some(StackValue::Struct(ref mut items)) => {
                items.reverse();
            }
            _ => {
                self.fault("REVERSEITEMS: not an array or struct");
            }
        }
    }

    /// Removes all items from the collection at the top of the stack.
    pub fn clear_items(&mut self) {
        let collection = self.stack.last_mut();
        match collection {
            Some(StackValue::Array(ref mut items)) | Some(StackValue::Struct(ref mut items)) => {
                items.clear();
            }
            Some(StackValue::Map(ref mut pairs)) => {
                pairs.clear();
            }
            _ => {
                self.fault("CLEARITEMS: not a collection");
            }
        }
    }

    /// Pops the last item from the array at the top of the stack and pushes it.
    pub fn pop_item(&mut self) {
        // We need to pop from the array, then push the popped value.
        // Get the item first by working on the top of stack.
        let item = {
            let collection = self.stack.last_mut();
            match collection {
                Some(StackValue::Array(ref mut items)) => {
                    if items.is_empty() {
                        None
                    } else {
                        Some(items.pop().unwrap())
                    }
                }
                _ => {
                    self.fault("POPITEM: not an array");
                    return;
                }
            }
        };
        match item {
            Some(val) => self.push(val),
            None => self.fault("POPITEM: array is empty"),
        }
    }

    // ---------------------------------------------------------------
    // Aliases for InstructionTranslator compatibility
    // ---------------------------------------------------------------

    /// Alias for new_array_0 (translator emits `ctx.new_array0()`)
    pub fn new_array0(&mut self) {
        self.new_array_0();
    }

    /// Alias for new_struct_0 (translator emits `ctx.new_struct0()`)
    pub fn new_struct0(&mut self) {
        self.new_struct_0();
    }

    /// Pops count, then that many key-value pairs, and pushes a Map (NeoVM PACKSTRUCT)
    pub fn pack_struct(&mut self) {
        let count = self.pop_integer();
        #[allow(clippy::cast_sign_loss)]
        let n = count as usize;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(self.pop());
        }
        items.reverse();
        self.push(StackValue::Struct(items));
    }

    /// Pops count, then that many key-value pairs, and pushes a Map (NeoVM PACKMAP)
    pub fn pack_map(&mut self) {
        let count = self.pop_integer();
        #[allow(clippy::cast_sign_loss)]
        let n = count as usize;
        let mut pairs = Vec::with_capacity(n);
        for _ in 0..n {
            let value = self.pop();
            let key = self.pop();
            pairs.push((key, value));
        }
        pairs.reverse();
        self.push(StackValue::Map(pairs));
    }
}

/// Returns the default `StackValue` for a NeoVM type tag.
fn default_for_type(type_byte: u8) -> StackValue {
    match type_byte {
        TAG_ARRAY => StackValue::Null,
        TAG_STRUCT => StackValue::Null,
        TAG_MAP => StackValue::Null,
        TAG_BUFFER => StackValue::Null,
        TAG_BYTESTRING => StackValue::Null,
        _ => StackValue::Null,
    }
}

#[cfg(test)]
mod tests {
    use crate::stack_value::StackValue;
    use crate::Context;

    fn ctx() -> Context {
        Context::from_abi_stack(vec![])
    }

    #[test]
    fn new_array_ops() {
        let mut c = ctx();
        c.new_array_0();
        assert_eq!(c.pop(), StackValue::Array(vec![]));

        c.push_int(3);
        c.new_array();
        let arr = c.pop();
        assert_eq!(
            arr,
            StackValue::Array(vec![StackValue::Null, StackValue::Null, StackValue::Null])
        );
    }

    #[test]
    fn new_struct_ops() {
        let mut c = ctx();
        c.new_struct_0();
        assert_eq!(c.pop(), StackValue::Struct(vec![]));

        c.push_int(2);
        c.new_struct();
        let s = c.pop();
        assert_eq!(
            s,
            StackValue::Struct(vec![StackValue::Null, StackValue::Null])
        );
    }

    #[test]
    fn new_map_and_new_buffer() {
        let mut c = ctx();
        c.new_map();
        assert_eq!(c.pop(), StackValue::Map(vec![]));

        c.push_int(4);
        c.new_buffer();
        assert_eq!(c.pop(), StackValue::Buffer(vec![0, 0, 0, 0]));
    }

    #[test]
    fn append_and_size() {
        let mut c = ctx();
        c.new_array_0();
        c.push_int(42);
        c.append();
        c.push_int(99);
        c.append();

        // Dup so we can check size then pick
        c.dup();
        c.size();
        assert_eq!(c.pop(), StackValue::Integer(2));
    }

    #[test]
    fn set_item_and_pick_item() {
        let mut c = ctx();
        c.push_int(3);
        c.new_array();

        // set_item: array[1] = 42
        c.push_int(1);
        c.push_int(42);
        c.set_item();

        // pick_item: get array[1]
        c.push_int(1);
        c.pick_item();
        assert_eq!(c.pop(), StackValue::Integer(42));
    }

    #[test]
    fn map_operations() {
        let mut c = ctx();
        c.new_map();

        // set_item on map
        c.push(StackValue::ByteString(b"key1".to_vec()));
        c.push_int(100);
        c.set_item();

        // has_key
        c.dup();
        c.push(StackValue::ByteString(b"key1".to_vec()));
        c.has_key();
        assert_eq!(c.pop(), StackValue::Boolean(true));

        // pick_item
        c.dup();
        c.push(StackValue::ByteString(b"key1".to_vec()));
        c.pick_item();
        assert_eq!(c.pop(), StackValue::Integer(100));

        // keys
        c.dup();
        c.keys();
        let keys = c.pop();
        assert_eq!(
            keys,
            StackValue::Array(vec![StackValue::ByteString(b"key1".to_vec())])
        );

        // values
        c.values();
        let vals = c.pop();
        assert_eq!(vals, StackValue::Array(vec![StackValue::Integer(100)]));
    }

    #[test]
    fn pack_and_unpack() {
        let mut c = ctx();
        c.push_int(10);
        c.push_int(20);
        c.push_int(30);
        c.push_int(3);
        c.pack();

        let packed = c.stack.last().cloned().unwrap();
        assert_eq!(
            packed,
            StackValue::Array(vec![
                StackValue::Integer(10),
                StackValue::Integer(20),
                StackValue::Integer(30),
            ])
        );

        c.unpack();
        let count = c.pop();
        assert_eq!(count, StackValue::Integer(3));
        assert_eq!(c.pop(), StackValue::Integer(30));
        assert_eq!(c.pop(), StackValue::Integer(20));
        assert_eq!(c.pop(), StackValue::Integer(10));
    }

    #[test]
    fn reverse_and_clear_items() {
        let mut c = ctx();
        c.new_array_0();
        c.push_int(1);
        c.append();
        c.push_int(2);
        c.append();
        c.push_int(3);
        c.append();

        c.reverse_items();
        c.dup();
        c.push_int(0);
        c.pick_item();
        assert_eq!(c.pop(), StackValue::Integer(3));

        c.clear_items();
        c.size();
        assert_eq!(c.pop(), StackValue::Integer(0));
    }

    #[test]
    fn pop_item_op() {
        let mut c = ctx();
        c.new_array_0();
        c.push_int(1);
        c.append();
        c.push_int(2);
        c.append();

        c.pop_item();
        assert_eq!(c.pop(), StackValue::Integer(2));
    }

    #[test]
    fn aliases_work() {
        let mut c = ctx();
        c.new_array0();
        c.push_int(1);
        c.append();
        c.size();
        assert_eq!(c.pop(), StackValue::Integer(1));

        c.new_struct0();
        assert!(matches!(c.pop(), StackValue::Struct(_)));
    }

    #[test]
    fn remove_from_array() {
        let mut c = ctx();
        c.new_array_0();
        c.push_int(10);
        c.append();
        c.push_int(20);
        c.append();
        c.push_int(30);
        c.append();

        c.push_int(1);
        c.remove();

        c.size();
        assert_eq!(c.pop(), StackValue::Integer(2));
    }
}
