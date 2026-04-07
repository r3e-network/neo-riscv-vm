//! Comparison and logic operations for the NeoVM `Context`.

use alloc::format;
use crate::stack_value::StackValue;
use crate::Context;

impl Context {
    // ---------------------------------------------------------------
    // Comparison
    // ---------------------------------------------------------------

    /// Pops two values and pushes `true` if they are not equal.
    pub fn not_equal(&mut self) {
        let b = self.pop();
        let a = self.pop();
        self.push_bool(a != b);
    }

    /// Pops two integers and pushes `true` if a < b.
    pub fn less_than(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a < b);
    }

    /// Pops two integers and pushes `true` if a <= b.
    pub fn less_or_equal(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a <= b);
    }

    /// Pops two integers and pushes `true` if a > b.
    pub fn greater_than(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a > b);
    }

    /// Pops two integers and pushes `true` if a >= b.
    pub fn greater_or_equal(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a >= b);
    }

    /// Pops two integers and pushes `true` if they are equal (numeric comparison).
    pub fn num_equal(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a == b);
    }

    /// Pops two integers and pushes `true` if they are not equal (numeric comparison).
    pub fn num_not_equal(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_bool(a != b);
    }

    // ---------------------------------------------------------------
    // Logic
    // ---------------------------------------------------------------

    /// Pops two booleans and pushes their logical AND.
    pub fn bool_and(&mut self) {
        let b = self.pop_bool_value();
        let a = self.pop_bool_value();
        self.push_bool(a && b);
    }

    /// Pops two booleans and pushes their logical OR.
    pub fn bool_or(&mut self) {
        let b = self.pop_bool_value();
        let a = self.pop_bool_value();
        self.push_bool(a || b);
    }

    /// Pops a boolean and pushes its logical NOT.
    pub fn not(&mut self) {
        let a = self.pop_bool_value();
        self.push_bool(!a);
    }

    /// Pops a value and pushes `true` if it is non-zero / truthy.
    pub fn nz(&mut self) {
        let v = self.pop_bool_value();
        self.push_bool(v);
    }

    /// Pops a value and pushes `true` if it is Null.
    pub fn is_null(&mut self) {
        let v = self.pop();
        self.push_bool(v == StackValue::Null);
    }

    // ---------------------------------------------------------------
    // Pop helpers that return Rust booleans (for generated branch code)
    // ---------------------------------------------------------------

    /// Pops the top value and returns it as a Rust `bool`.
    pub fn pop_bool(&mut self) -> bool {
        self.pop_bool_value()
    }

    /// Pops two values and returns `true` if they are equal.
    pub fn pop_cmp_eq(&mut self) -> bool {
        let b = self.pop();
        let a = self.pop();
        a == b
    }

    /// Pops two values and returns `true` if they are not equal.
    pub fn pop_cmp_ne(&mut self) -> bool {
        let b = self.pop();
        let a = self.pop();
        a != b
    }

    /// Pops two integers and returns `true` if a > b.
    pub fn pop_cmp_gt(&mut self) -> bool {
        let b = self.pop_integer();
        let a = self.pop_integer();
        a > b
    }

    /// Pops two integers and returns `true` if a >= b.
    pub fn pop_cmp_ge(&mut self) -> bool {
        let b = self.pop_integer();
        let a = self.pop_integer();
        a >= b
    }

    /// Pops two integers and returns `true` if a < b.
    pub fn pop_cmp_lt(&mut self) -> bool {
        let b = self.pop_integer();
        let a = self.pop_integer();
        a < b
    }

    /// Pops two integers and returns `true` if a <= b.
    pub fn pop_cmp_le(&mut self) -> bool {
        let b = self.pop_integer();
        let a = self.pop_integer();
        a <= b
    }

    // ---------------------------------------------------------------
    // Internal helper
    // ---------------------------------------------------------------

    /// Pops the top of the stack and coerces it to a Rust `bool`.
    ///
    /// - `Boolean(v)` -> v
    /// - `Integer(v)` -> v != 0
    /// - `Null` -> false
    /// - Other types -> fault
    fn pop_bool_value(&mut self) -> bool {
        match self.pop() {
            StackValue::Boolean(v) => v,
            StackValue::Integer(v) => v != 0,
            StackValue::Null => false,
            other => {
                self.fault(&format!(
                    "expected Boolean/Integer on stack for bool coercion, got tag {}",
                    other.type_tag()
                ));
                false
            }
        }
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
