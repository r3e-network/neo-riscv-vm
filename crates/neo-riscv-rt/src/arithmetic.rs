//! Arithmetic and bitwise operations for the NeoVM `Context`.

use crate::Context;

impl Context {
    // ---------------------------------------------------------------
    // Remaining arithmetic (integer fast path)
    // ---------------------------------------------------------------

    /// Pops two integers and pushes a / b (truncated toward zero).
    pub fn div(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        if b == 0 {
            self.fault("DIV: division by zero");
            return;
        }
        self.push_int(a.wrapping_div(b));
    }

    /// Pops two integers and pushes a % b.
    pub fn modulo(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        if b == 0 {
            self.fault("MOD: division by zero");
            return;
        }
        self.push_int(a.wrapping_rem(b));
    }

    /// Pops one integer and pushes its negation.
    pub fn negate(&mut self) {
        let a = self.pop_integer();
        self.push_int(a.wrapping_neg());
    }

    /// Pops one integer and pushes its absolute value.
    pub fn abs_val(&mut self) {
        let a = self.pop_integer();
        self.push_int(a.wrapping_abs());
    }

    /// Pops one integer and pushes its sign (-1, 0, or 1).
    pub fn sign(&mut self) {
        let a = self.pop_integer();
        self.push_int(a.signum());
    }

    /// Pops two integers and pushes the larger one.
    pub fn max(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a.max(b));
    }

    /// Pops two integers and pushes the smaller one.
    pub fn min(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a.min(b));
    }

    /// Pops exponent then base and pushes base^exponent.
    pub fn pow(&mut self) {
        let exp = self.pop_integer();
        let base = self.pop_integer();
        if exp < 0 {
            self.fault("POW: negative exponent");
            return;
        }
        // Clamp exponent to avoid unreasonably large computations.
        if exp > 63 {
            self.fault("POW: exponent too large for i64 fast path");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let result = base.wrapping_pow(exp as u32);
        self.push_int(result);
    }

    /// Pops one integer and pushes its integer square root.
    pub fn sqrt(&mut self) {
        let a = self.pop_integer();
        if a < 0 {
            self.fault("SQRT: negative value");
            return;
        }
        let r = isqrt(a as u64) as i64;
        self.push_int(r);
    }

    /// Pops modulus, then b, then a, and pushes (a * b) % modulus.
    pub fn modmul(&mut self) {
        let modulus = self.pop_integer();
        let b = self.pop_integer();
        let a = self.pop_integer();
        if modulus == 0 {
            self.fault("MODMUL: division by zero");
            return;
        }
        // Use i128 to avoid overflow in the intermediate product.
        let result = ((a as i128) * (b as i128)) % (modulus as i128);
        #[allow(clippy::cast_possible_truncation)]
        let result = result as i64;
        self.push_int(result);
    }

    /// Pops modulus, then exponent, then base, and pushes base^exponent % modulus.
    pub fn modpow(&mut self) {
        let modulus = self.pop_integer();
        let exp = self.pop_integer();
        let base = self.pop_integer();
        if modulus == 0 {
            self.fault("MODPOW: division by zero");
            return;
        }
        if exp < 0 {
            self.fault("MODPOW: negative exponent");
            return;
        }
        let result = mod_pow_i64(base, exp, modulus);
        self.push_int(result);
    }

    // ---------------------------------------------------------------
    // Shift operations
    // ---------------------------------------------------------------

    /// Pops shift amount then value and pushes value << shift.
    pub fn shl(&mut self) {
        let shift = self.pop_integer();
        let value = self.pop_integer();
        if shift < 0 || shift >= 64 {
            self.fault("SHL: shift amount out of range");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        self.push_int(value.wrapping_shl(shift as u32));
    }

    /// Pops shift amount then value and pushes value >> shift (arithmetic).
    pub fn shr(&mut self) {
        let shift = self.pop_integer();
        let value = self.pop_integer();
        if shift < 0 || shift >= 64 {
            self.fault("SHR: shift amount out of range");
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        self.push_int(value.wrapping_shr(shift as u32));
    }

    // ---------------------------------------------------------------
    // Bitwise operations
    // ---------------------------------------------------------------

    /// Pops two integers and pushes their bitwise AND.
    pub fn bitwise_and(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a & b);
    }

    /// Pops two integers and pushes their bitwise OR.
    pub fn bitwise_or(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a | b);
    }

    /// Pops two integers and pushes their bitwise XOR.
    pub fn bitwise_xor(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        self.push_int(a ^ b);
    }

    /// Pops one integer and pushes its bitwise NOT.
    pub fn bitwise_not(&mut self) {
        let a = self.pop_integer();
        self.push_int(!a);
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
        let a = self.pop_integer();
        self.push_int(a.wrapping_add(1));
    }

    /// Pops one integer and pushes value - 1 (NeoVM DEC)
    pub fn dec(&mut self) {
        let a = self.pop_integer();
        self.push_int(a.wrapping_sub(1));
    }

    /// Pops b, a, x and pushes (a <= x < b) (NeoVM WITHIN)
    pub fn within(&mut self) {
        let b = self.pop_integer();
        let a = self.pop_integer();
        let x = self.pop_integer();
        self.push_bool(x >= a && x < b);
    }
}

/// Modular exponentiation: base^exp % modulus, using i128 to avoid overflow.
fn mod_pow_i64(mut base: i64, mut exp: i64, modulus: i64) -> i64 {
    if modulus == 1 || modulus == -1 {
        return 0;
    }
    let m = modulus as i128;
    let mut result: i128 = 1;
    base = ((base as i128) % m) as i64;
    let mut b = base as i128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result * b) % m;
        }
        exp >>= 1;
        b = (b * b) % m;
    }
    result as i64
}

/// Integer square root using Newton's method (no floating point).
fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
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
