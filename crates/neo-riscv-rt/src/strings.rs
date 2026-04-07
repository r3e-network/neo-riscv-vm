//! String and byte-array operations for the NeoVM `Context`.

use crate::stack_value::StackValue;
use crate::Context;

impl Context {
    /// Pops two byte strings and pushes their concatenation.
    pub fn cat(&mut self) {
        let b = self.pop();
        let a = self.pop();
        match (a, b) {
            (StackValue::ByteString(mut a_bytes), StackValue::ByteString(b_bytes)) => {
                a_bytes.extend_from_slice(&b_bytes);
                self.push(StackValue::ByteString(a_bytes));
            }
            (StackValue::Buffer(mut a_buf), StackValue::Buffer(b_buf)) => {
                a_buf.extend_from_slice(&b_buf);
                self.push(StackValue::Buffer(a_buf));
            }
            (StackValue::ByteString(mut a_bytes), StackValue::Buffer(b_buf)) => {
                a_bytes.extend_from_slice(&b_buf);
                self.push(StackValue::ByteString(a_bytes));
            }
            (StackValue::Buffer(mut a_buf), StackValue::ByteString(b_bytes)) => {
                a_buf.extend_from_slice(&b_bytes);
                self.push(StackValue::Buffer(a_buf));
            }
            _ => {
                self.fault("CAT: operands must be ByteString or Buffer");
            }
        }
    }

    /// Pops count, index, then a byte string and pushes the substring.
    pub fn substr(&mut self) {
        let count = self.pop_integer();
        let index = self.pop_integer();
        let val = self.pop();

        let bytes = match &val {
            StackValue::ByteString(b) => b,
            StackValue::Buffer(b) => b,
            _ => {
                self.fault("SUBSTR: not a ByteString or Buffer");
                return;
            }
        };

        if index < 0 || count < 0 {
            self.fault("SUBSTR: negative index or count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let (index, count) = (index as usize, count as usize);

        if index + count > bytes.len() {
            self.fault("SUBSTR: range out of bounds");
            return;
        }

        let sub = bytes[index..index + count].to_vec();
        match val {
            StackValue::Buffer(_) => self.push(StackValue::Buffer(sub)),
            _ => self.push(StackValue::ByteString(sub)),
        }
    }

    /// Pops count then a byte string and pushes the first `count` bytes.
    pub fn left(&mut self) {
        let count = self.pop_integer();
        let val = self.pop();

        let bytes = match &val {
            StackValue::ByteString(b) => b,
            StackValue::Buffer(b) => b,
            _ => {
                self.fault("LEFT: not a ByteString or Buffer");
                return;
            }
        };

        if count < 0 {
            self.fault("LEFT: negative count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        if count > bytes.len() {
            self.fault("LEFT: count exceeds length");
            return;
        }

        let sub = bytes[..count].to_vec();
        match val {
            StackValue::Buffer(_) => self.push(StackValue::Buffer(sub)),
            _ => self.push(StackValue::ByteString(sub)),
        }
    }

    /// Pops count then a byte string and pushes the last `count` bytes.
    pub fn right(&mut self) {
        let count = self.pop_integer();
        let val = self.pop();

        let bytes = match &val {
            StackValue::ByteString(b) => b,
            StackValue::Buffer(b) => b,
            _ => {
                self.fault("RIGHT: not a ByteString or Buffer");
                return;
            }
        };

        if count < 0 {
            self.fault("RIGHT: negative count");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        if count > bytes.len() {
            self.fault("RIGHT: count exceeds length");
            return;
        }

        let start = bytes.len() - count;
        let sub = bytes[start..].to_vec();
        match val {
            StackValue::Buffer(_) => self.push(StackValue::Buffer(sub)),
            _ => self.push(StackValue::ByteString(sub)),
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

        let src_bytes = match &src {
            StackValue::ByteString(b) | StackValue::Buffer(b) => b,
            _ => {
                self.fault("MEMCPY: source is not a ByteString or Buffer");
                return;
            }
        };

        if count < 0 || si < 0 || di < 0 {
            self.fault("MEMCPY: negative argument");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let (count, si, di) = (count as usize, si as usize, di as usize);

        if si + count > src_bytes.len() {
            self.fault("MEMCPY: source range out of bounds");
            return;
        }

        let src_slice = src_bytes[si..si + count].to_vec();

        let dst = self.stack.last_mut();
        match dst {
            Some(StackValue::Buffer(ref mut buf)) => {
                if di + count > buf.len() {
                    self.fault("MEMCPY: destination range out of bounds");
                    return;
                }
                buf[di..di + count].copy_from_slice(&src_slice);
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
    fn substr_op() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello world".to_vec()));
        c.push_int(6); // index
        c.push_int(5); // count
        c.substr();
        assert_eq!(c.pop(), StackValue::ByteString(b"world".to_vec()));
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
    fn right_op() {
        let mut c = ctx();
        c.push(StackValue::ByteString(b"hello".to_vec()));
        c.push_int(3);
        c.right();
        assert_eq!(c.pop(), StackValue::ByteString(b"llo".to_vec()));
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
