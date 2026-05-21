//! Generated-code compatibility methods backed by shared NeoVM semantics.
//!
//! The C# to RISC-V compiler emits `Context` method calls. This module keeps
//! that stable surface while delegating opcode behavior to `neo-vm-rs`.

use crate::Context;
use neo_riscv_abi::semantics::runtime::{
    arithmetic as vm_arithmetic, collections as vm_collections, comparison as vm_comparison,
    conversion as vm_conversion,
};

impl Context {
    pub fn div(&mut self) {
        vm_arithmetic::div(self);
    }

    pub fn modulo(&mut self) {
        vm_arithmetic::modulo(self);
    }

    pub fn negate(&mut self) {
        vm_arithmetic::negate(self);
    }

    pub fn abs_val(&mut self) {
        vm_arithmetic::abs(self);
    }

    pub fn sign(&mut self) {
        vm_arithmetic::sign(self);
    }

    pub fn max(&mut self) {
        vm_arithmetic::max(self);
    }

    pub fn min(&mut self) {
        vm_arithmetic::min(self);
    }

    pub fn pow(&mut self) {
        vm_arithmetic::pow(self);
    }

    pub fn sqrt(&mut self) {
        vm_arithmetic::sqrt(self);
    }

    pub fn modmul(&mut self) {
        vm_arithmetic::modmul(self);
    }

    pub fn modpow(&mut self) {
        vm_arithmetic::modpow(self);
    }

    pub fn shl(&mut self) {
        vm_arithmetic::shl(self);
    }

    pub fn shr(&mut self) {
        vm_arithmetic::shr(self);
    }

    pub fn bitwise_and(&mut self) {
        vm_arithmetic::bitwise_and(self);
    }

    pub fn bitwise_or(&mut self) {
        vm_arithmetic::bitwise_or(self);
    }

    pub fn bitwise_xor(&mut self) {
        vm_arithmetic::bitwise_xor(self);
    }

    pub fn bitwise_not(&mut self) {
        vm_arithmetic::bitwise_not(self);
    }

    pub fn abs(&mut self) {
        self.abs_val();
    }

    pub fn mod_mul(&mut self) {
        self.modmul();
    }

    pub fn mod_pow(&mut self) {
        self.modpow();
    }

    pub fn inc(&mut self) {
        vm_arithmetic::inc(self);
    }

    pub fn dec(&mut self) {
        vm_arithmetic::dec(self);
    }

    pub fn within(&mut self) {
        vm_arithmetic::within(self);
    }

    pub fn not_equal(&mut self) {
        vm_comparison::not_equal(self);
    }

    pub fn less_than(&mut self) {
        vm_comparison::less_than(self);
    }

    pub fn less_or_equal(&mut self) {
        vm_comparison::less_or_equal(self);
    }

    pub fn greater_than(&mut self) {
        vm_comparison::greater_than(self);
    }

    pub fn greater_or_equal(&mut self) {
        vm_comparison::greater_or_equal(self);
    }

    pub fn num_equal(&mut self) {
        vm_comparison::num_equal(self);
    }

    pub fn num_not_equal(&mut self) {
        vm_comparison::num_not_equal(self);
    }

    pub fn bool_and(&mut self) {
        vm_comparison::bool_and(self);
    }

    pub fn bool_or(&mut self) {
        vm_comparison::bool_or(self);
    }

    pub fn not(&mut self) {
        vm_comparison::not(self);
    }

    pub fn nz(&mut self) {
        vm_comparison::nz(self);
    }

    pub fn is_null(&mut self) {
        vm_comparison::is_null(self);
    }

    pub fn pop_bool(&mut self) -> bool {
        vm_comparison::pop_bool(self)
    }

    pub fn pop_cmp_eq(&mut self) -> bool {
        vm_comparison::pop_cmp_eq(self)
    }

    pub fn pop_cmp_ne(&mut self) -> bool {
        vm_comparison::pop_cmp_ne(self)
    }

    pub fn pop_cmp_gt(&mut self) -> bool {
        vm_comparison::pop_cmp_gt(self)
    }

    pub fn pop_cmp_ge(&mut self) -> bool {
        vm_comparison::pop_cmp_ge(self)
    }

    pub fn pop_cmp_lt(&mut self) -> bool {
        vm_comparison::pop_cmp_lt(self)
    }

    pub fn pop_cmp_le(&mut self) -> bool {
        vm_comparison::pop_cmp_le(self)
    }

    pub fn is_type(&mut self, type_byte: u8) {
        vm_conversion::is_type(self, type_byte);
    }

    pub fn convert_to(&mut self, target_type: u8) {
        vm_conversion::convert_to(self, target_type);
    }

    pub fn push_bigint(&mut self, bytes: &[u8]) {
        vm_conversion::push_bigint(self, bytes);
    }

    pub fn push_default(&mut self, type_byte: u8) {
        vm_conversion::push_default(self, type_byte);
    }

    pub fn new_array_0(&mut self) {
        vm_collections::new_array_0(self);
    }

    pub fn new_array(&mut self) {
        vm_collections::new_array(self);
    }

    pub fn new_array_t(&mut self, type_byte: u8) {
        vm_collections::new_array_t(self, type_byte);
    }

    pub fn new_struct_0(&mut self) {
        vm_collections::new_struct_0(self);
    }

    pub fn new_struct(&mut self) {
        vm_collections::new_struct(self);
    }

    pub fn new_map(&mut self) {
        vm_collections::new_map(self);
    }

    pub fn new_buffer(&mut self) {
        vm_collections::new_buffer(self);
    }

    pub fn append(&mut self) {
        vm_collections::append(self);
    }

    pub fn set_item(&mut self) {
        vm_collections::set_item(self);
    }

    pub fn pick_item(&mut self) {
        vm_collections::pick_item(self);
    }

    pub fn remove(&mut self) {
        vm_collections::remove(self);
    }

    pub fn size(&mut self) {
        vm_collections::size(self);
    }

    pub fn has_key(&mut self) {
        vm_collections::has_key(self);
    }

    pub fn keys(&mut self) {
        vm_collections::keys(self);
    }

    pub fn values(&mut self) {
        vm_collections::values(self);
    }

    pub fn pack(&mut self) {
        vm_collections::pack(self);
    }

    pub fn unpack(&mut self) {
        vm_collections::unpack(self);
    }

    pub fn reverse_items(&mut self) {
        vm_collections::reverse_items(self);
    }

    pub fn clear_items(&mut self) {
        vm_collections::clear_items(self);
    }

    pub fn pop_item(&mut self) {
        vm_collections::pop_item(self);
    }

    pub fn new_array0(&mut self) {
        self.new_array_0();
    }

    pub fn new_struct0(&mut self) {
        self.new_struct_0();
    }

    pub fn pack_struct(&mut self) {
        vm_collections::pack_struct(self);
    }

    pub fn pack_map(&mut self) {
        vm_collections::pack_map(self);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, StackValue};

    fn ctx() -> Context {
        Context::from_abi_stack(vec![])
    }

    #[test]
    fn arithmetic_aliases_delegate_to_shared_vm() {
        let mut c = ctx();
        c.push_int(2);
        c.push_int(10);
        c.pow();
        assert_eq!(c.pop(), StackValue::Integer(1024));

        c.push_int(-7);
        c.abs();
        assert_eq!(c.pop(), StackValue::Integer(7));

        c.push_int(7);
        c.push_int(8);
        c.push_int(10);
        c.mod_mul();
        assert_eq!(c.pop(), StackValue::Integer(6));
    }

    #[test]
    fn comparison_helpers_delegate_to_shared_vm() {
        let mut c = ctx();
        c.push_int(5);
        c.push_int(3);
        assert!(c.pop_cmp_gt());

        c.push_bool(true);
        c.push_bool(false);
        c.bool_or();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn conversion_and_collection_ops_delegate_to_shared_vm() {
        let mut c = ctx();
        c.push_bool(true);
        c.convert_to(0x21);
        assert_eq!(c.pop(), StackValue::Integer(1));

        c.push_int(2);
        c.new_array_t(0x28);
        assert_eq!(
            c.pop(),
            StackValue::Array(vec![
                StackValue::ByteString(Vec::new()),
                StackValue::ByteString(Vec::new()),
            ])
        );

        c.new_array0();
        c.push_int(42);
        c.append();
        c.size();
        assert_eq!(c.pop(), StackValue::Integer(1));
    }
}
