//! Arithmetic and bitwise operations for the NeoVM `Context`.

use crate::Context;
use neo_riscv_abi::semantics::runtime::arithmetic as vm_arithmetic;

impl Context {
    // ---------------------------------------------------------------
    // Remaining arithmetic (integer fast path)
    // ---------------------------------------------------------------

    /// Pops two integers and pushes a / b (truncated toward zero).
    pub fn div(&mut self) {
        vm_arithmetic::div(self);
    }

    /// Pops two integers and pushes a % b.
    pub fn modulo(&mut self) {
        vm_arithmetic::modulo(self);
    }

    /// Pops one integer and pushes its negation.
    pub fn negate(&mut self) {
        vm_arithmetic::negate(self);
    }

    /// Pops one integer and pushes its absolute value.
    pub fn abs_val(&mut self) {
        vm_arithmetic::abs(self);
    }

    /// Pops one integer and pushes its sign (-1, 0, or 1).
    pub fn sign(&mut self) {
        vm_arithmetic::sign(self);
    }

    /// Pops two integers and pushes the larger one.
    pub fn max(&mut self) {
        vm_arithmetic::max(self);
    }

    /// Pops two integers and pushes the smaller one.
    pub fn min(&mut self) {
        vm_arithmetic::min(self);
    }

    /// Pops exponent then base and pushes base^exponent.
    pub fn pow(&mut self) {
        vm_arithmetic::pow(self);
    }

    /// Pops one integer and pushes its integer square root.
    pub fn sqrt(&mut self) {
        vm_arithmetic::sqrt(self);
    }

    /// Pops modulus, then b, then a, and pushes (a * b) % modulus.
    pub fn modmul(&mut self) {
        vm_arithmetic::modmul(self);
    }

    /// Pops modulus, then exponent, then base, and pushes base^exponent % modulus.
    pub fn modpow(&mut self) {
        vm_arithmetic::modpow(self);
    }

    // ---------------------------------------------------------------
    // Shift operations
    // ---------------------------------------------------------------

    /// Pops shift amount then value and pushes value << shift.
    pub fn shl(&mut self) {
        vm_arithmetic::shl(self);
    }

    /// Pops shift amount then value and pushes value >> shift (arithmetic).
    pub fn shr(&mut self) {
        vm_arithmetic::shr(self);
    }

    // ---------------------------------------------------------------
    // Bitwise operations
    // ---------------------------------------------------------------

    /// Pops two integers and pushes their bitwise AND.
    pub fn bitwise_and(&mut self) {
        vm_arithmetic::bitwise_and(self);
    }

    /// Pops two integers and pushes their bitwise OR.
    pub fn bitwise_or(&mut self) {
        vm_arithmetic::bitwise_or(self);
    }

    /// Pops two integers and pushes their bitwise XOR.
    pub fn bitwise_xor(&mut self) {
        vm_arithmetic::bitwise_xor(self);
    }

    /// Pops one integer and pushes its bitwise NOT.
    pub fn bitwise_not(&mut self) {
        vm_arithmetic::bitwise_not(self);
    }

    // ---------------------------------------------------------------
    // Aliases and additional ops for InstructionTranslator compatibility
    // ---------------------------------------------------------------

    /// Alias for abs_val (translator emits `ctx.abs()`)
    pub fn abs(&mut self) {
        self.abs_val();
    }

    /// Alias for modmul (translator emits `ctx.mod_mul()`)
    pub fn mod_mul(&mut self) {
        self.modmul();
    }

    /// Alias for modpow (translator emits `ctx.mod_pow()`)
    pub fn mod_pow(&mut self) {
        self.modpow();
    }

    /// Pops one integer and pushes value + 1 (NeoVM INC)
    pub fn inc(&mut self) {
        vm_arithmetic::inc(self);
    }

    /// Pops one integer and pushes value - 1 (NeoVM DEC)
    pub fn dec(&mut self) {
        vm_arithmetic::dec(self);
    }

    /// Pops b, a, x and pushes (a <= x < b) (NeoVM WITHIN)
    pub fn within(&mut self) {
        vm_arithmetic::within(self);
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
    fn div_and_modulo() {
        let mut c = ctx();
        c.push_int(10);
        c.push_int(3);
        c.div();
        assert_eq!(c.pop(), StackValue::Integer(3));

        c.push_int(10);
        c.push_int(3);
        c.modulo();
        assert_eq!(c.pop(), StackValue::Integer(1));
    }

    #[test]
    fn div_by_zero_faults() {
        let mut c = ctx();
        c.push_int(10);
        c.push_int(0);
        c.div();
        assert!(c.fault_message.is_some());
    }

    #[test]
    fn negate_abs_sign() {
        let mut c = ctx();
        c.push_int(5);
        c.negate();
        assert_eq!(c.pop(), StackValue::Integer(-5));

        c.push_int(-7);
        c.abs_val();
        assert_eq!(c.pop(), StackValue::Integer(7));

        c.push_int(-3);
        c.sign();
        assert_eq!(c.pop(), StackValue::Integer(-1));

        c.push_int(0);
        c.sign();
        assert_eq!(c.pop(), StackValue::Integer(0));

        c.push_int(10);
        c.sign();
        assert_eq!(c.pop(), StackValue::Integer(1));
    }

    #[test]
    fn max_and_min() {
        let mut c = ctx();
        c.push_int(3);
        c.push_int(7);
        c.max();
        assert_eq!(c.pop(), StackValue::Integer(7));

        c.push_int(3);
        c.push_int(7);
        c.min();
        assert_eq!(c.pop(), StackValue::Integer(3));
    }

    #[test]
    fn pow_op() {
        let mut c = ctx();
        c.push_int(2);
        c.push_int(10);
        c.pow();
        assert_eq!(c.pop(), StackValue::Integer(1024));
    }

    #[test]
    fn sqrt_op() {
        let mut c = ctx();
        c.push_int(49);
        c.sqrt();
        assert_eq!(c.pop(), StackValue::Integer(7));
    }

    #[test]
    fn modmul_op() {
        let mut c = ctx();
        c.push_int(7);
        c.push_int(8);
        c.push_int(10);
        c.modmul();
        assert_eq!(c.pop(), StackValue::Integer(6)); // (7*8) % 10 = 56 % 10 = 6
    }

    #[test]
    fn modpow_op() {
        let mut c = ctx();
        c.push_int(2);
        c.push_int(10);
        c.push_int(100);
        c.modpow();
        assert_eq!(c.pop(), StackValue::Integer(24)); // 2^10 % 100 = 1024 % 100 = 24
    }

    #[test]
    fn shl_shr() {
        let mut c = ctx();
        c.push_int(1);
        c.push_int(4);
        c.shl();
        assert_eq!(c.pop(), StackValue::Integer(16));

        c.push_int(16);
        c.push_int(2);
        c.shr();
        assert_eq!(c.pop(), StackValue::Integer(4));
    }

    #[test]
    fn bitwise_ops() {
        let mut c = ctx();
        c.push_int(0b1100);
        c.push_int(0b1010);
        c.bitwise_and();
        assert_eq!(c.pop(), StackValue::Integer(0b1000));

        c.push_int(0b1100);
        c.push_int(0b1010);
        c.bitwise_or();
        assert_eq!(c.pop(), StackValue::Integer(0b1110));

        c.push_int(0b1100);
        c.push_int(0b1010);
        c.bitwise_xor();
        assert_eq!(c.pop(), StackValue::Integer(0b0110));

        c.push_int(0);
        c.bitwise_not();
        assert_eq!(c.pop(), StackValue::Integer(-1));
    }
}
