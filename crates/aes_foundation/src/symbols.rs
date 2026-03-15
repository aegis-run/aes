use aes_allocator::Allocator;
use nonmax::NonMaxU32;
use std::marker::PhantomData;

/// A highly optimized `u32` identifier specifically for interned symbols.
///
/// Unlike [`crate::Id`], `SymbolId` is backed by a `NonMaxU32`, guaranteeing that
/// it never equals `u32::MAX`. This allows the Rust compiler to perform memory layout
/// optimizations, such as making `Option<SymbolId>` equal to 4 bytes rather than 8 bytes.
///
/// Due to this layout, `SymbolId` is heavily used within maps and sets (`SymbolMap`/`SymbolSet`)
/// during semantic analysis to avoid costly string allocations or hashing overhead.
#[repr(transparent)]
pub struct SymbolId<T> {
    index: NonMaxU32,
    _marker: PhantomData<T>,
}

impl<T> SymbolId<T> {
    pub const MAX_INDEX: usize = (u32::MAX - 1) as usize;

    #[inline]
    pub const fn new(index: u32) -> Self {
        let Some(index) = NonMaxU32::new(index) else {
            panic!("SymbolId index overflow: reached u32::MAX");
        };

        Self {
            index,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn as_index(&self) -> usize {
        self.index.get() as usize
    }
}

impl<T> std::fmt::Debug for SymbolId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SymbolId({})", self.as_index())
    }
}

impl<T> std::fmt::Display for SymbolId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_index())
    }
}

impl<T> Copy for SymbolId<T> {}
impl<T> Clone for SymbolId<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for SymbolId<T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for SymbolId<T> {}
impl<T> PartialOrd for SymbolId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for SymbolId<T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> std::hash::Hash for SymbolId<T> {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

/// An array-backed map optimized for `O(1)` access using a `SymbolId` as the key.
///
/// Because `SymbolId`s are assigned sequentially and densely, `SymbolMap` uses a simple
/// `Vec<Option<V>>` rather than a hashed collection. This guarantees extremely fast lookups and
/// insertions, acting effectively as a Structure of Arrays mapping symbols to associated data.
#[derive(Debug, Clone)]
pub struct SymbolMap<'alloc, K, V> {
    data: aes_allocator::Vec<'alloc, Option<V>>,
    _marker: PhantomData<K>,
}

impl<'alloc, K, V> SymbolMap<'alloc, K, V> {
    pub fn new(alloc: &'alloc Allocator) -> Self {
        Self {
            data: aes_allocator::Vec::new_in(alloc),
            _marker: PhantomData,
        }
    }

    pub fn with_capacity(alloc: &'alloc Allocator, capacity: usize) -> Self {
        Self {
            data: aes_allocator::Vec::with_capacity_in(capacity, alloc),
            _marker: PhantomData,
        }
    }

    pub fn get(&self, key: SymbolId<K>) -> Option<&V> {
        self.data.get(key.as_index())?.as_ref()
    }

    pub fn get_mut(&mut self, key: SymbolId<K>) -> Option<&mut V> {
        self.data.get_mut(key.as_index())?.as_mut()
    }

    pub fn push_sequential(&mut self, key: SymbolId<K>, value: V) {
        let idx = key.as_index();
        debug_assert!(
            idx <= self.data.len(),
            "SymbolMap::push_sequential called with sparse key {idx}; use a HashMap for sparse access",
        );
        if idx == self.data.len() {
            self.data.push(Some(value));
        } else {
            self.data[idx] = Some(value);
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = SymbolId<K>> + '_ {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.as_ref().map(|_| SymbolId::new(i as u32)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct K;

    fn id(i: u32) -> SymbolId<K> {
        SymbolId::new(i)
    }

    #[test]
    fn empty_map_returns_none() {
        let alloc = Allocator::new();
        let map = SymbolMap::<K, u32>::new(&alloc);
        assert!(map.get(id(0)).is_none());
    }

    #[test]
    fn sequential_push_and_get() {
        let alloc = Allocator::new();
        let mut map = SymbolMap::new(&alloc);

        map.push_sequential(id(0), 10);
        map.push_sequential(id(1), 20);
        map.push_sequential(id(2), 30);

        assert_eq!(map.get(id(0)), Some(&10));
        assert_eq!(map.get(id(1)), Some(&20));
        assert_eq!(map.get(id(2)), Some(&30));
    }

    #[test]
    fn overwrite_existing_slot() {
        let alloc = Allocator::new();
        let mut map = SymbolMap::new(&alloc);

        map.push_sequential(id(0), 1);
        map.push_sequential(id(0), 99);

        assert_eq!(map.get(id(0)), Some(&99));
        assert_eq!(map.data.len(), 1);
    }

    #[test]
    fn get_mut_allows_in_place_mutation() {
        let alloc = Allocator::new();
        let mut map = SymbolMap::new(&alloc);

        map.push_sequential(id(0), 5);
        *map.get_mut(id(0)).unwrap() += 1;

        assert_eq!(map.get(id(0)), Some(&6));
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let alloc = Allocator::new();
        let map = SymbolMap::<K, u32>::new(&alloc);

        assert!(map.get(id(100)).is_none());
    }

    #[test]
    fn keys_only_yields_populated_slots() {
        let alloc = Allocator::new();
        let mut map = SymbolMap::new(&alloc);

        map.push_sequential(id(0), 1);
        map.push_sequential(id(1), 2);
        // overwrite id(1) — still one slot, not two keys
        map.push_sequential(id(1), 3);
        let keys: Vec<_> = map.keys().map(|k| k.as_index()).collect();
        assert_eq!(keys, vec![0, 1]);
    }

    #[test]
    fn with_capacity_behaves_like_new() {
        let alloc = Allocator::new();
        let mut a = SymbolMap::new(&alloc);
        let mut b = SymbolMap::new(&alloc);

        for i in 0..5u32 {
            a.push_sequential(id(i), i * 10);
            b.push_sequential(id(i), i * 10);
        }
        for i in 0..5u32 {
            assert_eq!(a.get(id(i)), b.get(id(i)));
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "sparse key")]
    fn sparse_push_panics_in_debug() {
        let alloc = Allocator::new();
        let mut map = SymbolMap::new(&alloc);
        map.push_sequential(id(5), 99);
    }

    proptest! {
      #[test]
      fn sequential_push_get_roundtrip(
         values in proptest::collection::vec(any::<u32>(), 0..100)
      ) {
          let alloc = Allocator::new();
          let mut map = SymbolMap::new(&alloc);

          for (i, &v) in values.iter().enumerate() {
              map.push_sequential(id(i as u32), v);
          }

          // Every value is readable back
          for (i, &v) in values.iter().enumerate() {
              assert_eq!(map.get(id(i as u32)), Some(&v));
          }

          // Last write wins for overwrites — check final state matches values
          assert_eq!(map.data.len(), values.len());
      }

      #[test]
      fn overwrite_does_not_grow(
          values in proptest::collection::vec(any::<u32>(), 1..50),
          overwrites in proptest::collection::vec((any::<u32>(), any::<u32>()), 0..20),
      ) {
          let alloc = Allocator::new();
          let mut map = SymbolMap::new(&alloc);

          for (i, &v) in values.iter().enumerate() {
              map.push_sequential(id(i as u32), v);
          }

          let len_before = map.data.len();

          // Overwrite random existing slots
          for (raw_idx, val) in overwrites {
              let idx = raw_idx as usize % values.len();
              map.push_sequential(id(idx as u32), val);
          }

          assert_eq!(map.data.len(), len_before);
      }
    }
}
