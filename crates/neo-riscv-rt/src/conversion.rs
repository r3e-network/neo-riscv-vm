//! Type conversion and introspection operations for the NeoVM `Context`.

use crate::Context;
use neo_riscv_abi::semantics::runtime::conversion as vm_conversion;

impl Context {
    /// Pops a value and pushes `true` if its type tag matches `type_byte`.
    pub fn is_type(&mut self, type_byte: u8) {
        vm_conversion::is_type(self, type_byte);
    }

    /// Converts the top stack value to the NeoVM type indicated by `target_type`.
    ///
    /// This replaces the existing `convert` stub with a more complete implementation.
    pub fn convert_to(&mut self, target_type: u8) {
        vm_conversion::convert_to(self, target_type);
    }

    /// Pushes a `BigInteger` onto the stack from raw little-endian two's complement bytes.
    pub fn push_bigint(&mut self, bytes: &[u8]) {
        vm_conversion::push_bigint(self, bytes);
    }

    /// Pushes the default value for the given NeoVM type tag.
    pub fn push_default(&mut self, type_byte: u8) {
        vm_conversion::push_default(self, type_byte);
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
    fn is_type_op() {
        let mut c = ctx();
        c.push_int(42);
        c.is_type(0); // TAG_INTEGER
        assert_eq!(c.pop(), StackValue::Boolean(true));

        c.push_bool(true);
        c.is_type(0);
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn convert_bool_to_int() {
        let mut c = ctx();
        c.push_bool(true);
        c.convert_to(0); // TAG_INTEGER
        assert_eq!(c.pop(), StackValue::Integer(1));
    }

    #[test]
    fn convert_int_to_bytestring() {
        let mut c = ctx();
        c.push_int(256);
        c.convert_to(2); // TAG_BYTESTRING
        let val = c.pop();
        match val {
            StackValue::ByteString(bytes) => {
                assert_eq!(bytes, vec![0, 1]); // 256 LE = 0x00, 0x01
            }
            other => panic!("expected ByteString, got {:?}", other),
        }
    }

    #[test]
    fn convert_bytestring_to_int() {
        let mut c = ctx();
        c.push(StackValue::ByteString(vec![0, 1]));
        c.convert_to(0); // TAG_INTEGER
        assert_eq!(c.pop(), StackValue::Integer(256));
    }

    #[test]
    fn push_bigint_op() {
        let mut c = ctx();
        c.push_bigint(&[0xFF, 0x00]);
        assert_eq!(c.pop(), StackValue::BigInteger(vec![0xFF, 0x00]));
    }

    #[test]
    fn push_default_types() {
        let mut c = ctx();
        c.push_default(0); // INTEGER
        assert_eq!(c.pop(), StackValue::Integer(0));

        c.push_default(1); // BOOLEAN
        assert_eq!(c.pop(), StackValue::Boolean(false));

        c.push_default(7); // NULL
        assert_eq!(c.pop(), StackValue::Null);
    }

    #[test]
    fn convert_array_to_struct() {
        let mut c = ctx();
        c.push(StackValue::Array(vec![StackValue::Integer(1)]));
        c.convert_to(5); // TAG_STRUCT
        match c.pop() {
            StackValue::Struct(items) => {
                assert_eq!(items, vec![StackValue::Integer(1)]);
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn convert_negative_int_to_bytestring() {
        let mut c = ctx();
        c.push_int(-1);
        c.convert_to(2); // TAG_BYTESTRING
        let val = c.pop();
        match val {
            StackValue::ByteString(bytes) => {
                assert_eq!(bytes, vec![0xFF]);
            }
            other => panic!("expected ByteString, got {:?}", other),
        }
    }

    #[test]
    fn convert_int_to_neovm_bytestring_tag() {
        let mut c = ctx();
        c.push_int(256);
        c.convert_to(0x28); // NeoVM ByteString type tag
        let val = c.pop();
        match val {
            StackValue::ByteString(bytes) => {
                assert_eq!(bytes, vec![0, 1]);
            }
            other => panic!("expected ByteString, got {:?}", other),
        }
    }

    #[test]
    fn convert_null_preserves_null() {
        let mut c = ctx();
        c.push(StackValue::Null);
        c.convert_to(0x28); // NeoVM ByteString type tag
        assert_eq!(c.pop(), StackValue::Null);

        c.push(StackValue::Null);
        c.convert_to(0x30); // NeoVM Buffer type tag
        assert_eq!(c.pop(), StackValue::Null);
    }

    #[test]
    fn convert_primitives_to_buffer_uses_shared_rules() {
        let mut c = ctx();
        c.push_bool(true);
        c.convert_to(0x30); // NeoVM Buffer type tag
        assert_eq!(c.pop(), StackValue::Buffer(vec![1]));

        c.push(StackValue::BigInteger(vec![0xff, 0x00]));
        c.convert_to(0x30);
        assert_eq!(c.pop(), StackValue::Buffer(vec![0xff, 0x00]));
    }
}
