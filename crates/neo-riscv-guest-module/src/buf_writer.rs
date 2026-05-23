pub(crate) struct BufWriter {
    buf: &'static mut [u8],
    len: usize,
}

impl BufWriter {
    pub(crate) fn new(buf: &'static mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

impl core::fmt::Write for BufWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len().saturating_sub(self.len);
        let copy = bytes.len().min(remaining);
        if copy > 0 {
            self.buf[self.len..self.len + copy].copy_from_slice(&bytes[..copy]);
            self.len += copy;
        }
        Ok(())
    }
}
