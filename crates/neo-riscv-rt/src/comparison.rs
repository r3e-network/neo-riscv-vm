//! Comparison and logic operations for the NeoVM `Context`.

use crate::Context;
use neo_riscv_abi::semantics::runtime::comparison as vm_comparison;

impl Context {
    // ---------------------------------------------------------------
    // Comparison
    // ---------------------------------------------------------------

    /// Pops two values and pushes `true` if they are not equal.
    pub fn not_equal(&mut self) {
        vm_comparison::not_equal(self);
    }

    /// Pops two integers and pushes `true` if a < b.
    pub fn less_than(&mut self) {
        vm_comparison::less_than(self);
    }

    /// Pops two integers and pushes `true` if a <= b.
    pub fn less_or_equal(&mut self) {
        vm_comparison::less_or_equal(self);
    }

    /// Pops two integers and pushes `true` if a > b.
    pub fn greater_than(&mut self) {
        vm_comparison::greater_than(self);
    }

    /// Pops two integers and pushes `true` if a >= b.
    pub fn greater_or_equal(&mut self) {
        vm_comparison::greater_or_equal(self);
    }

    /// Pops two integers and pushes `true` if they are equal (numeric comparison).
    pub fn num_equal(&mut self) {
        vm_comparison::num_equal(self);
    }

    /// Pops two integers and pushes `true` if they are not equal (numeric comparison).
    pub fn num_not_equal(&mut self) {
        vm_comparison::num_not_equal(self);
    }

    // ---------------------------------------------------------------
    // Logic
    // ---------------------------------------------------------------

    /// Pops two booleans and pushes their logical AND.
    pub fn bool_and(&mut self) {
        vm_comparison::bool_and(self);
    }

    /// Pops two booleans and pushes their logical OR.
    pub fn bool_or(&mut self) {
        vm_comparison::bool_or(self);
    }

    /// Pops a boolean and pushes its logical NOT.
    pub fn not(&mut self) {
        vm_comparison::not(self);
    }

    /// Pops a value and pushes `true` if it is non-zero / truthy.
    pub fn nz(&mut self) {
        vm_comparison::nz(self);
    }

    /// Pops a value and pushes `true` if it is Null.
    pub fn is_null(&mut self) {
        vm_comparison::is_null(self);
    }

    // ---------------------------------------------------------------
    // Pop helpers that return Rust booleans (for generated branch code)
    // ---------------------------------------------------------------

    /// Pops the top value and returns it as a Rust `bool`.
    pub fn pop_bool(&mut self) -> bool {
        vm_comparison::pop_bool(self)
    }

    /// Pops two values and returns `true` if they are equal.
    pub fn pop_cmp_eq(&mut self) -> bool {
        vm_comparison::pop_cmp_eq(self)
    }

    /// Pops two values and returns `true` if they are not equal.
    pub fn pop_cmp_ne(&mut self) -> bool {
        vm_comparison::pop_cmp_ne(self)
    }

    /// Pops two integers and returns `true` if a > b.
    pub fn pop_cmp_gt(&mut self) -> bool {
        vm_comparison::pop_cmp_gt(self)
    }

    /// Pops two integers and returns `true` if a >= b.
    pub fn pop_cmp_ge(&mut self) -> bool {
        vm_comparison::pop_cmp_ge(self)
    }

    /// Pops two integers and returns `true` if a < b.
    pub fn pop_cmp_lt(&mut self) -> bool {
        vm_comparison::pop_cmp_lt(self)
    }

    /// Pops two integers and returns `true` if a <= b.
    pub fn pop_cmp_le(&mut self) -> bool {
        vm_comparison::pop_cmp_le(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::Context;
    use crate::StackValue;

    fn ctx() -> Context {
        Context::from_abi_stack(vec![])
    }

    #[test]
    fn not_equal_different() {
        let mut c = ctx();
        c.push_int(1);
        c.push_int(2);
        c.not_equal();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn not_equal_same() {
        let mut c = ctx();
        c.push_int(5);
        c.push_int(5);
        c.not_equal();
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn less_than_ops() {
        let mut c = ctx();
        c.push_int(3);
        c.push_int(5);
        c.less_than();
        assert_eq!(c.pop(), StackValue::Boolean(true));

        c.push_int(5);
        c.push_int(3);
        c.less_than();
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn less_or_equal_ops() {
        let mut c = ctx();
        c.push_int(5);
        c.push_int(5);
        c.less_or_equal();
        assert_eq!(c.pop(), StackValue::Boolean(true));

        c.push_int(6);
        c.push_int(5);
        c.less_or_equal();
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn greater_than_ops() {
        let mut c = ctx();
        c.push_int(5);
        c.push_int(3);
        c.greater_than();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn greater_or_equal_ops() {
        let mut c = ctx();
        c.push_int(5);
        c.push_int(5);
        c.greater_or_equal();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn bool_and_or() {
        let mut c = ctx();
        c.push_bool(true);
        c.push_bool(false);
        c.bool_and();
        assert_eq!(c.pop(), StackValue::Boolean(false));

        c.push_bool(true);
        c.push_bool(false);
        c.bool_or();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn not_op() {
        let mut c = ctx();
        c.push_bool(true);
        c.not();
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn nz_op() {
        let mut c = ctx();
        c.push_int(0);
        c.nz();
        assert_eq!(c.pop(), StackValue::Boolean(false));

        c.push_int(42);
        c.nz();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }

    #[test]
    fn is_null_op() {
        let mut c = ctx();
        c.push_null();
        c.is_null();
        assert_eq!(c.pop(), StackValue::Boolean(true));

        c.push_int(1);
        c.is_null();
        assert_eq!(c.pop(), StackValue::Boolean(false));
    }

    #[test]
    fn pop_cmp_helpers() {
        let mut c = ctx();
        c.push_int(1);
        c.push_int(1);
        assert!(c.pop_cmp_eq());

        c.push_int(1);
        c.push_int(2);
        assert!(c.pop_cmp_ne());

        c.push_int(5);
        c.push_int(3);
        assert!(c.pop_cmp_gt());

        c.push_int(5);
        c.push_int(5);
        assert!(c.pop_cmp_ge());

        c.push_int(3);
        c.push_int(5);
        assert!(c.pop_cmp_lt());

        c.push_int(5);
        c.push_int(5);
        assert!(c.pop_cmp_le());
    }

    #[test]
    fn num_equal_and_not_equal() {
        let mut c = ctx();
        c.push_int(10);
        c.push_int(10);
        c.num_equal();
        assert_eq!(c.pop(), StackValue::Boolean(true));

        c.push_int(10);
        c.push_int(11);
        c.num_not_equal();
        assert_eq!(c.pop(), StackValue::Boolean(true));
    }
}
