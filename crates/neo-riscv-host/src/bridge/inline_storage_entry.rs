use super::{INLINE_KEY_CAP, INLINE_VALUE_CAP};

pub(super) struct InlineStorageEntry {
    key_len: u8,
    key: [u8; INLINE_KEY_CAP],
    value_len: u16,
    value: [u8; INLINE_VALUE_CAP],
}

impl InlineStorageEntry {
    pub(super) fn new(key: &[u8], value: &[u8]) -> Option<Self> {
        if key.len() > INLINE_KEY_CAP || value.len() > INLINE_VALUE_CAP {
            return None;
        }

        let mut key_buf = [0u8; INLINE_KEY_CAP];
        key_buf[..key.len()].copy_from_slice(key);
        let mut value_buf = [0u8; INLINE_VALUE_CAP];
        value_buf[..value.len()].copy_from_slice(value);

        Some(Self {
            key_len: key.len() as u8,
            key: key_buf,
            value_len: value.len() as u16,
            value: value_buf,
        })
    }

    pub(super) fn key(&self) -> &[u8] {
        &self.key[..self.key_len as usize]
    }

    pub(super) fn value(&self) -> &[u8] {
        &self.value[..self.value_len as usize]
    }

    pub(super) fn matches(&self, key: &[u8]) -> bool {
        self.key() == key
    }

    pub(super) fn overwrite_value(&mut self, value: &[u8]) -> bool {
        if value.len() > INLINE_VALUE_CAP {
            return false;
        }
        self.value[..value.len()].copy_from_slice(value);
        self.value_len = value.len() as u16;
        true
    }
}
