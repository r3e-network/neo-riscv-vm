/// `core::fmt::Write` over a fixed buffer with truncation detection for the
/// guest module's panic handler.
///
/// The standard library's `&mut [u8]` Writer (stable since Rust 1.64) silently
/// returns `Err` on overflow — it cannot mark truncation. This implementation
/// writes `...[TRUNCATED]` at the end of the buffer when the message exceeds
/// available space, so truncated panic messages are visually distinguishable
/// from complete ones during debugging.
///
/// The toolchain is now `stable` (per `rust-toolchain.toml`), so the standard
/// writer IS available. It is intentionally NOT used because the truncation
/// marker is load-bearing for guest debugging.
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
        if bytes.len() <= remaining {
            self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
            self.len += bytes.len();
        } else {
            // Buffer is full; copy what fits, leaving room for the truncation marker.
            const TRUNCATED_MARKER: &[u8] = b"...[TRUNCATED]";
            let marker_end = self.buf.len();
            let marker_start = marker_end.saturating_sub(TRUNCATED_MARKER.len());
            let available_for_data = marker_start.saturating_sub(self.len);
            if available_for_data > 0 {
                let copy = bytes.len().min(available_for_data);
                self.buf[self.len..self.len + copy].copy_from_slice(&bytes[..copy]);
                self.len += copy;
            }
            // Always write the truncation marker at the end of the buffer,
            // overwriting any previous marker or trailing data.
            let actual_marker_start = self.buf.len().saturating_sub(TRUNCATED_MARKER.len());
            self.buf[actual_marker_start..].copy_from_slice(TRUNCATED_MARKER);
            self.len = self.buf.len();
        }
        Ok(())
    }
}
