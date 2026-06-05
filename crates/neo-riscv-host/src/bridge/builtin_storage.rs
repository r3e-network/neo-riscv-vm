use std::collections::BTreeMap;

use super::{
    INLINE_ENTRY_SLOTS, SMALL_ENTRY_SLOTS, inline_storage_entry::InlineStorageEntry,
    small_storage_entry::SmallStorageEntry,
};

/// Maximum number of entries allowed in the heap storage to prevent memory exhaustion.
const MAX_HEAP_ENTRIES: usize = 1024;

pub(super) struct BuiltinStorage {
    pub(super) hot_small: Option<SmallStorageEntry>,
    small: [Option<SmallStorageEntry>; SMALL_ENTRY_SLOTS],
    inline: [Option<InlineStorageEntry>; INLINE_ENTRY_SLOTS],
    heap: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl BuiltinStorage {
    pub(super) fn new() -> Self {
        Self {
            hot_small: None,
            small: std::array::from_fn(|_| None),
            inline: std::array::from_fn(|_| None),
            heap: BTreeMap::new(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn get(&self, key: &[u8]) -> Option<&[u8]> {
        if let Some(entry) = &self.hot_small
            && entry.matches(key)
        {
            return Some(entry.value());
        }
        for entry in self.small.iter().flatten() {
            if entry.matches(key) {
                return Some(entry.value());
            }
        }
        for entry in self.inline.iter().flatten() {
            if entry.matches(key) {
                return Some(entry.value());
            }
        }
        self.heap.get(key).map(Vec::as_slice)
    }

    pub(super) fn get_promoting(&mut self, key: &[u8]) -> Option<&[u8]> {
        if self
            .hot_small
            .as_ref()
            .is_some_and(|entry| entry.matches(key))
        {
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        if let Some(index) = self.find_small_index(key) {
            let promoted = {
                let entry = self.small[index]
                    .as_ref()
                    .expect("small slot index must reference an entry");
                SmallStorageEntry::new(entry.key(), entry.value())
            };
            self.hot_small = promoted;
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        if let Some(index) = self.find_inline_index(key) {
            let promoted = {
                let entry = self.inline[index]
                    .as_ref()
                    .expect("inline slot index must reference an entry");
                SmallStorageEntry::new(entry.key(), entry.value())
            };
            if let Some(entry) = promoted {
                self.hot_small = Some(entry);
                return self.hot_small.as_ref().map(|entry| entry.value());
            }
            return self.inline[index].as_ref().map(|entry| entry.value());
        }

        if let Some(entry) = self
            .heap
            .get(key)
            .and_then(|value| SmallStorageEntry::new(key, value))
        {
            self.hot_small = Some(entry);
            return self.hot_small.as_ref().map(|entry| entry.value());
        }

        self.heap.get(key).map(Vec::as_slice)
    }

    pub(super) fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.refresh_hot_small(key, value);

        if let Some(index) = self.find_small_index(key) {
            if self.small[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return;
            }
            self.small[index] = None;
        }

        if let Some(entry) = SmallStorageEntry::new(key, value)
            && let Some(slot) = self.small.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(entry);
            self.remove_from_inline_only(key);
            self.heap.remove(key);
            return;
        }

        if let Some(index) = self.find_inline_index(key) {
            if self.inline[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.heap.remove(key);
                return;
            }
            self.inline[index] = None;
        }

        if let Some(entry) = InlineStorageEntry::new(key, value)
            && let Some(slot) = self.inline.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(entry);
            self.heap.remove(key);
            return;
        }

        // Check if we've reached the maximum heap entries limit.
        // Evict the first key in sorted (BTreeMap) order to maintain deterministic
        // behavior across nodes — critical for blockchain consensus.
        if !self.heap.contains_key(key)
            && self.heap.len() >= MAX_HEAP_ENTRIES
            && let Some(first_key) = self.heap.keys().next()
        {
            let key_to_remove = first_key.clone();
            self.heap.remove(&key_to_remove);
        }

        self.heap.insert(key.to_vec(), value.to_vec());
    }

    pub(super) fn put_small_inline(&mut self, key: &[u8], value: &[u8]) -> bool {
        self.refresh_hot_small(key, value);

        if let Some(index) = self.find_small_index(key) {
            if self.small[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.remove_from_inline_only(key);
                self.heap.remove(key);
                return true;
            }
            self.small[index] = None;
        }

        if let Some(entry) = SmallStorageEntry::new(key, value)
            && let Some(slot) = self.small.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(entry);
            self.remove_from_inline_only(key);
            self.heap.remove(key);
            return true;
        }

        if let Some(index) = self.find_inline_index(key) {
            if self.inline[index]
                .as_mut()
                .is_some_and(|entry| entry.overwrite_value(value))
            {
                self.heap.remove(key);
                return true;
            }
            self.inline[index] = None;
        }

        if let Some(entry) = InlineStorageEntry::new(key, value)
            && let Some(slot) = self.inline.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(entry);
            self.heap.remove(key);
            return true;
        }
        false
    }

    fn find_small_index(&self, key: &[u8]) -> Option<usize> {
        self.small
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|entry| entry.matches(key)))
    }

    fn find_inline_index(&self, key: &[u8]) -> Option<usize> {
        self.inline
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|entry| entry.matches(key)))
    }

    fn refresh_hot_small(&mut self, key: &[u8], value: &[u8]) {
        match SmallStorageEntry::new(key, value) {
            Some(entry) => self.hot_small = Some(entry),
            None if self
                .hot_small
                .as_ref()
                .is_some_and(|entry| entry.matches(key)) =>
            {
                self.hot_small = None;
            }
            None => {}
        }
    }

    pub(super) fn remove(&mut self, key: &[u8]) {
        if self
            .hot_small
            .as_ref()
            .is_some_and(|entry| entry.matches(key))
        {
            self.hot_small = None;
        }
        for slot in &mut self.small {
            if slot.as_ref().is_some_and(|entry| entry.matches(key)) {
                *slot = None;
                return;
            }
        }
        self.remove_from_inline_only(key);
        self.heap.remove(key);
    }

    fn remove_from_inline_only(&mut self, key: &[u8]) {
        for slot in &mut self.inline {
            if slot.as_ref().is_some_and(|entry| entry.matches(key)) {
                *slot = None;
                return;
            }
        }
    }
}
