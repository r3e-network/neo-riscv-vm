//! String and byte-array operations for the NeoVM `Context`.

use crate::stack_value::{
    byte_sequence_bytes, byte_sequence_len, concat_byte_sequences, slice_byte_sequence, StackValue,
};
use crate::Context;

impl Context {
    /// Pops two byte strings and pushes their concatenation.
    pub fn cat(&mut self) {
        let b = self.pop();
        let a = self.pop();
        match concat_byte_sequences(a, b) {
            Some(value) => self.push(value),
            None => self.fault("CAT: operands must be ByteString or Buffer"),
        }
    }

    /// Pops count, index, then a byte string and pushes the substring.
    pub fn substr(&mut self) {
        let count = self.pop_integer();
        let index = self.pop_integer();
        let val = self.pop();

        let Some(len) = byte_sequence_len(&val) else {
            self.fault("SUBSTR: not a ByteString or Buffer");
            return;
        };

        if index < 0 || count < 0 {
            self.fault("SUBSTR: negative index or count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let (index, count) = (index as usize, count as usize);

        let Some(end) = index.checked_add(count) else {
            self.fault("SUBSTR: range out of bounds");
            return;
        };
        if end > len {
            self.fault("SUBSTR: range out of bounds");
            return;
        }

        match slice_byte_sequence(val, index, count) {
            Some(value) => self.push(value),
            None => self.fault("SUBSTR: range out of bounds"),
        }
    }

    /// Pops count then a byte string and pushes the first `count` bytes.
    pub fn left(&mut self) {
        let count = self.pop_integer();
        let val = self.pop();

        let Some(len) = byte_sequence_len(&val) else {
            self.fault("LEFT: not a ByteString or Buffer");
            return;
        };

        if count < 0 {
            self.fault("LEFT: negative count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        if count > len {
            self.fault("LEFT: count exceeds length");
            return;
        }

        match slice_byte_sequence(val, 0, count) {
            Some(value) => self.push(value),
            None => self.fault("LEFT: count exceeds length"),
        }
    }

    /// Pops count then a byte string and pushes the last `count` bytes.
    pub fn right(&mut self) {
        let count = self.pop_integer();
        let val = self.pop();

        let Some(len) = byte_sequence_len(&val) else {
            self.fault("RIGHT: not a ByteString or Buffer");
            return;
        };

        if count < 0 {
            self.fault("RIGHT: negative count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        if count > len {
            self.fault("RIGHT: count exceeds length");
            return;
        }

        match slice_byte_sequence(val, len - count, count) {
            Some(value) => self.push(value),
            None => self.fault("RIGHT: count exceeds length"),
        }
    }

    /// Pops count, src_index, src_buffer, dst_index, and copies into the
    /// dst buffer at the top of the stack.
    ///
    /// Stack order (top first): count, si, src, di, [dst is on stack already]
    pub fn memcpy(&mut self) {
        let count = self.pop_integer();
        let si = self.pop_integer();
        let src = self.pop();
        let di = self.pop_integer();

        let Some(src_bytes) = byte_sequence_bytes(&src) else {
            self.fault("MEMCPY: source is not a ByteString or Buffer");
            return;
        };

        if count < 0 || si < 0 || di < 0 {
            self.fault("MEMCPY: negative argument");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let (count, si, di) = (count as usize, si as usize, di as usize);

        let Some(src_end) = si.checked_add(count) else {
            self.fault("MEMCPY: source range out of bounds");
            return;
        };
        if src_end > src_bytes.len() {
            self.fault("MEMCPY: source range out of bounds");
            return;
        }

        let src_slice = src_bytes[si..src_end].to_vec();

        let dst = self.stack.last_mut();
        match dst {
            Some(StackValue::Buffer(ref mut buf)) => {
                let Some(dst_end) = di.checked_add(count) else {
                    self.fault("MEMCPY: destination range out of bounds");
                    return;
                };
                if dst_end > buf.len() {
                    self.fault("MEMCPY: destination range out of bounds");
                    return;
                }
                buf[di..dst_end].copy_from_slice(&src_slice);
            }
            _ => {
                self.fault("MEMCPY: destination is not a Buffer");
            }
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
    fn cat_bytestrings() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello".to_vec()));
        c.push(StackValue::ByteString(b" world".to_vec()));
        c.cat();
        assert_eq!(c.pop(), StackValue::ByteString(b"hello world".to_vec()));
    }

    #[test]
    fn cat_preserves_left_byte_sequence_type() {
        let mut c = ctx();
        c.push(StackValue::Buffer(b"neo".to_vec()));
        c.push(StackValue::ByteString(b"n4".to_vec()));
        c.cat();
        assert_eq!(c.pop(), StackValue::Buffer(b"neon4".to_vec()));

        c.push(StackValue::ByteString(b"neo".to_vec()));
        c.push(StackValue::Buffer(b"n4".to_vec()));
        c.cat();
        assert_eq!(c.pop(), StackValue::ByteString(b"neon4".to_vec()));
    }

    #[test]
    fn substr_op() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello world".to_vec()));
        c.push_int(6); // index
        c.push_int(5); // count
        c.substr();
        assert_eq!(c.pop(), StackValue::ByteString(b"world".to_vec()));
    }

    #[test]
    fn substr_preserves_buffer_type() {
        let mut c = ctx();
        c.push(StackValue::Buffer(b"hello world".to_vec()));
        c.push_int(6); // index
        c.push_int(5); // count
        c.substr();
        assert_eq!(c.pop(), StackValue::Buffer(b"world".to_vec()));
    }

    #[test]
    fn left_op() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello".to_vec()));
        c.push_int(3);
        c.left();
        assert_eq!(c.pop(), StackValue::ByteString(b"hel".to_vec()));
    }

    #[test]
    fn left_preserves_buffer_type() {
        let mut c = ctx();
        c.push(StackValue::Buffer(b"hello".to_vec()));
        c.push_int(3);
        c.left();
        assert_eq!(c.pop(), StackValue::Buffer(b"hel".to_vec()));
    }

    #[test]
    fn right_op() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello".to_vec()));
        c.push_int(3);
        c.right();
        assert_eq!(c.pop(), StackValue::ByteString(b"llo".to_vec()));
    }

    #[test]
    fn right_preserves_buffer_type() {
        let mut c = ctx();
        c.push(StackValue::Buffer(b"hello".to_vec()));
        c.push_int(3);
        c.right();
        assert_eq!(c.pop(), StackValue::Buffer(b"llo".to_vec()));
    }

    #[test]
    fn memcpy_op() {
        let mut c = ctx();
        // dst buffer on stack
        c.push_int(10);
        c.new_buffer();

        // args: di, src, si, count
        c.push_int(2); // di
        c.push(StackValue::ByteString(b"ABCDEF".to_vec())); // src
        c.push_int(1); // si
        c.push_int(3); // count
        c.memcpy();

        let dst = c.pop();
        match dst {
            StackValue::Buffer(buf) => {
                assert_eq!(buf[2], b'B');
                assert_eq!(buf[3], b'C');
                assert_eq!(buf[4], b'D');
            }
            other => panic!("expected Buffer, got {:?}", other),
        }
    }
}
