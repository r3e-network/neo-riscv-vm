//! Collection operations for the NeoVM `Context`.
//!
//! Implements array, struct, map, and buffer creation and manipulation.

use crate::stack_value::StackValue;
use crate::Context;
use alloc::vec::Vec;
use neo_riscv_abi::semantics::collections as vm_collections;

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
        self.push_collection_result(vm_collections::new_array(count));
    }

    /// Pops a count and pushes a typed array (for now, all items are default for type).
    pub fn new_array_t(&mut self, type_byte: u8) {
        let count = self.pop_integer();
        self.push_collection_result(vm_collections::new_array_t(count, type_byte));
    }

    /// Pushes an empty struct onto the stack.
    pub fn new_struct_0(&mut self) {
        self.push(StackValue::Struct(Vec::new()));
    }

    /// Pops a count and pushes a struct of that many `Null` values.
    pub fn new_struct(&mut self) {
        let count = self.pop_integer();
        self.push_collection_result(vm_collections::new_struct(count));
    }

    /// Pushes an empty map onto the stack.
    pub fn new_map(&mut self) {
        self.push(StackValue::Map(Vec::new()));
    }

    /// Pops a size and pushes a zero-filled buffer of that size.
    pub fn new_buffer(&mut self) {
        let size = self.pop_integer();
        self.push_collection_result(vm_collections::new_buffer(size));
    }

    // ---------------------------------------------------------------
    // Collection operations
    // ---------------------------------------------------------------

    /// Pops a value then an array/struct/map and appends the value.
    pub fn append(&mut self) {
        let value = self.pop();
        let result = match self.stack.last_mut() {
            Some(collection) => vm_collections::append(collection, value),
            None => Err("APPEND: top-1 is not an array or struct".into()),
        };
        if let Err(message) = result {
            self.fault(&message);
        }
    }

    /// Pops value, key, then collection and sets collection[key] = value.
    pub fn set_item(&mut self) {
        let value = self.pop();
        let key = self.pop();
        let result = match self.stack.last_mut() {
            Some(collection) => vm_collections::set_item(collection, key, value),
            None => Err("SETITEM: not a collection".into()),
        };
        if let Err(message) = result {
            self.fault(&message);
        }
    }

    /// Pops key then collection and pushes collection[key].
    pub fn pick_item(&mut self) {
        let key = self.pop();
        let collection = self.pop();
        match vm_collections::pick_item(&collection, &key) {
            Ok(value) => self.push(value),
            Err(message) => self.fault(&message),
        }
    }

    /// Pops key then collection and removes the entry.
    pub fn remove(&mut self) {
        let key = self.pop();
        let result = match self.stack.last_mut() {
            Some(collection) => vm_collections::remove(collection, &key),
            None => Err("REMOVE: not a collection".into()),
        };
        if let Err(message) = result {
            self.fault(&message);
        }
    }

    /// Pops a collection/string/buffer and pushes its size.
    pub fn size(&mut self) {
        let val = self.pop();
        match vm_collections::size(&val) {
            Ok(size) => self.push_int(size),
            Err(message) => self.fault(&message),
        }
    }

    /// Pops key then collection and pushes `true` if the key exists.
    pub fn has_key(&mut self) {
        let key = self.pop();
        let collection = self.pop();
        match vm_collections::has_key(&collection, &key) {
            Ok(found) => self.push_bool(found),
            Err(message) => self.fault(&message),
        }
    }

    /// Pops a map and pushes an array of its keys.
    pub fn keys(&mut self) {
        let val = self.pop();
        match vm_collections::keys(val) {
            Ok(value) => self.push(value),
            Err(message) => self.fault(&message),
        }
    }

    /// Pops a map and pushes an array of its values.
    pub fn values(&mut self) {
        let val = self.pop();
        match vm_collections::values(val) {
            Ok(value) => self.push(value),
            Err(message) => self.fault(&message),
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
        self.push(vm_collections::pack(items));
    }

    /// Pops an array and pushes all its items then the count.
    pub fn unpack(&mut self) {
        let val = self.pop();
        match vm_collections::unpack(val) {
            Ok(values) => {
                for value in values {
                    self.push(value);
                }
            }
            Err(message) => self.fault(&message),
        }
    }

    /// Pops a collection and pushes it with items in reverse order.
    pub fn reverse_items(&mut self) {
        let result = match self.stack.last_mut() {
            Some(collection) => vm_collections::reverse_items(collection),
            None => Err("REVERSEITEMS: not an array or struct".into()),
        };
        if let Err(message) = result {
            self.fault(&message);
        }
    }

    /// Removes all items from the collection at the top of the stack.
    pub fn clear_items(&mut self) {
        let result = match self.stack.last_mut() {
            Some(collection) => vm_collections::clear_items(collection),
            None => Err("CLEARITEMS: not a collection".into()),
        };
        if let Err(message) = result {
            self.fault(&message);
        }
    }

    /// Pops the last item from the array at the top of the stack and pushes it.
    pub fn pop_item(&mut self) {
        match vm_collections::pop_item(self.pop()) {
            Ok(values) => {
                for value in values {
                    self.push(value);
                }
            }
            Err(message) => self.fault(&message),
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
        if count < 0 {
            self.fault("PACKSTRUCT: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let n = count as usize;
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(self.pop());
        }
        items.reverse();
        self.push(vm_collections::pack_struct(items));
    }

    /// Pops count, then that many key-value pairs, and pushes a Map (NeoVM PACKMAP)
    pub fn pack_map(&mut self) {
        let count = self.pop_integer();
        if count < 0 {
            self.fault("PACKMAP: negative count");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let n = count as usize;
        let mut pairs = Vec::with_capacity(n);
        for _ in 0..n {
            let value = self.pop();
            let key = self.pop();
            pairs.push((key, value));
        }
        pairs.reverse();
        self.push(vm_collections::pack_map(pairs));
    }

    fn push_collection_result(&mut self, result: Result<StackValue, String>) {
        match result {
            Ok(value) => self.push(value),
            Err(message) => self.fault(&message),
        }
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
    fn new_array_t_uses_neovm_default_values() {
        let mut c = ctx();
        c.push_int(2);
        c.new_array_t(0x21);
        assert_eq!(
            c.pop(),
            StackValue::Array(vec![StackValue::Integer(0), StackValue::Integer(0)])
        );

        c.push_int(2);
        c.new_array_t(0x28);
        assert_eq!(
            c.pop(),
            StackValue::Array(vec![
                StackValue::ByteString(Vec::new()),
                StackValue::ByteString(Vec::new()),
            ])
        );

        c.push_int(2);
        c.new_array_t(0x20);
        assert_eq!(
            c.pop(),
            StackValue::Array(vec![StackValue::Null, StackValue::Null])
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
