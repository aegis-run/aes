use std::marker::PhantomData;

use rustc_hash::FxHashMap;

use crate::symbols::SymbolId;

/// A string deduplication utility that converts string slices into `O(1)` comparative `SymbolId`s.
///
/// The `Interner` maps identical string literals (e.g., type names, relation names)
/// to the exact same `SymbolId`. This allows the rest of the compiler to perform cheap
/// `u32` equality checks instead of expensive, repeated string comparisons.
///
/// Internally, it relies on an `FxHashMap` for fast hashing during the initial conversion
/// (`&str -> SymbolId`), and a simple `Vec<&str>` for instant resolution (`SymbolId -> &str`).
pub struct Interner<'src, T> {
    map: FxHashMap<&'src str, SymbolId<T>>,
    buf: Vec<&'src str>,
    _marker: PhantomData<T>,
}

impl<'src, T> Default for Interner<'src, T> {
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
            buf: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<'src, T> Clone for Interner<'src, T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            buf: self.buf.clone(),
            _marker: PhantomData,
        }
    }
}

impl<'src, T> std::fmt::Debug for Interner<'src, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interner")
            .field("map", &self.map)
            .field("buf", &self.buf)
            .finish()
    }
}

impl<'src, T> Interner<'src, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: FxHashMap::with_capacity_and_hasher(cap, Default::default()),
            buf: Vec::with_capacity(cap),
            _marker: PhantomData,
        }
    }

    pub fn intern(&mut self, s: &'src str) -> SymbolId<T> {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = SymbolId::new(self.buf.len() as u32);
        self.buf.push(s);
        self.map.insert(s, id);
        id
    }

    pub fn get(&self, s: &str) -> Option<SymbolId<T>> {
        self.map.get(s).copied()
    }

    pub fn resolve(&self, id: SymbolId<T>) -> &'src str {
        debug_assert!(
            id.as_index() < self.buf.len(),
            "SymbolId({}) out of bounds for interner of len {}",
            id.as_index(),
            self.buf.len(),
        );
        self.buf.get(id.as_index()).expect("SymbolId out of bounds")
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (SymbolId<T>, &'src str)> + '_ {
        self.buf
            .iter()
            .enumerate()
            .map(|(i, &s)| (SymbolId::new(i as u32), s))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use proptest::prelude::*;

    struct A;
    struct B;
    type TestInterner<'a> = Interner<'a, A>;

    #[test]
    fn empty_interner_has_zero_len() {
        let i = TestInterner::new();
        assert_eq!(i.len(), 0);
        assert!(i.is_empty());
    }

    #[test]
    fn interner_indexing_begins_at_zero() {
        let mut i = TestInterner::new();
        let id = i.intern("hello");

        assert_eq!(id.as_index(), 0);
        assert_eq!(i.len(), 1);
        assert!(!i.is_empty());
        assert_eq!(i.resolve(id), "hello");
    }

    #[test]
    fn duplicate_intern_returns_same_id() {
        let mut i = TestInterner::new();
        let id1 = i.intern("hello");
        let id2 = i.intern("hello");
        assert_eq!(id1, id2);
        assert_eq!(i.len(), 1);
        assert_eq!(i.resolve(id1), "hello");
        assert_eq!(i.resolve(id2), "hello");
    }

    #[test]
    fn interns_empty_string() {
        let mut i = TestInterner::new();
        let id = i.intern("");
        assert_eq!(id.as_index(), 0);
        assert_eq!(i.len(), 1);
        assert!(!i.is_empty());
        assert_eq!(i.resolve(id), "");
        assert_eq!(i.get(""), Some(id))
    }

    #[test]
    fn ids_order_matches_insertion_order() {
        let mut i = TestInterner::new();
        let words = ["alpha", "beta", "gamma", "delta"];
        for (expected_idx, word) in words.iter().enumerate() {
            let id = i.intern(word);
            assert_eq!(id.as_index(), expected_idx);
        }
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let i = TestInterner::new();
        assert_eq!(i.get("missing"), None);
    }

    #[test]
    fn iter_covers_every_interned_string() {
        let mut i = TestInterner::new();
        let words = ["x", "y", "z"];
        let ids: Vec<_> = words.iter().map(|w| i.intern(w)).collect();

        let pairs: Vec<_> = i.iter().collect();
        assert_eq!(pairs.len(), words.len());
        for ((id, sym), (&expected_id, &expected_sym)) in
            pairs.iter().zip(ids.iter().zip(words.iter()))
        {
            assert_eq!(*id, expected_id);
            assert_eq!(*sym, expected_sym);
        }
    }

    #[test]
    fn clone_is_independent() {
        let mut original = TestInterner::new();
        original.intern("shared");
        let mut cloned = original.clone();

        cloned.intern("only-in-clone");
        assert_eq!(original.len(), 1);
        assert_eq!(cloned.len(), 2);

        let id = original.get("shared").unwrap();
        assert_eq!(original.resolve(id), cloned.resolve(id));
    }

    #[test]
    fn with_capacity_behaves_like_new() {
        let mut a = TestInterner::new();
        let mut b = TestInterner::with_capacity(64);
        let words = ["one", "two", "three"];

        for word in &words {
            a.intern(word);
            b.intern(word);
        }
        assert_eq!(a.len(), b.len());

        for word in &words {
            assert_eq!(a.get(word), b.get(word));
        }
    }

    #[test]
    fn distinct_marker_types_are_type_incompatible() {
        let mut a: Interner<A> = Interner::new();
        let mut b: Interner<B> = Interner::new();
        let _id_a: SymbolId<A> = a.intern("hello");
        let _id_b: SymbolId<B> = b.intern("hello");
        // If this compiles without type errors, the phantom type is working.
        // _id_a and _id_b have different types: no mix-up is possible.
    }

    proptest! {
      #[test]
      fn intern_deduplication_and_resolve(
          strings in proptest::collection::vec("[a-z]{0,8}", 0..100)
      ) {
          let mut interner = TestInterner::new();
          let mut first_seen = HashMap::new();

          for s in &strings {
              let id = interner.intern(s.as_str());

              // Idempotence: same string always gives same id
              assert_eq!(interner.intern(s.as_str()), id);

              // Consistency with first observation
              match first_seen.entry(s.clone()) {
                  std::collections::hash_map::Entry::Occupied(e) => {
                      assert_eq!(id, *e.get(), "ID changed for {:?}", s);
                  }
                  std::collections::hash_map::Entry::Vacant(e) => {
                      e.insert(id);
                  }
              }

              // Roundtrip: resolve -> re-intern gives back the same id
              let resolved = interner.resolve(id);
              assert_eq!(resolved, s.as_str());
              assert_eq!(interner.intern(resolved), id);

              // get() agrees with intern()
              assert_eq!(interner.get(s.as_str()), Some(id));
          }

          // Length == number of unique strings
          let unique: HashSet<_> = strings.iter().collect();
          assert_eq!(interner.len(), unique.len());
      }

      #[test]
      fn ids_are_dense(
          strings in proptest::collection::vec("[a-z]{0,8}", 0..100)
      ) {
          let mut interner = TestInterner::new();
          for s in &strings {
              interner.intern(s.as_str());
          }

          // Every index in [0, len) must be resolvable and re-intern to itself
          for idx in 0..interner.len() {
              let id = SymbolId::<A>::new(idx as u32);
              let s = interner.resolve(id);
              assert_eq!(interner.intern(s), id);
          }
      }

      #[test]
      fn iter_is_complete_and_ordered(
          strings in proptest::collection::vec("[a-z]{0,8}", 0..100)
      ) {
          let mut interner = TestInterner::new();
          for s in &strings {
              interner.intern(s.as_str());
          }

          let pairs: Vec<_> = interner.iter().collect();
          assert_eq!(pairs.len(), interner.len());

          for (i, (id, s)) in pairs.iter().enumerate() {
              assert_eq!(id.as_index(), i);
              assert_eq!(interner.resolve(*id), *s);
          }
      }

      #[test]
      fn unicode_strings(
          strings in proptest::collection::vec(
              proptest::string::string_regex(
                  r"[\u{0000}-\u{FFFF}]{0,16}"
              ).unwrap(),
              0..50,
          )
      ) {
          let mut interner = TestInterner::new();
          let mut first_seen = HashMap::new();

          for s in &strings {
              let id = interner.intern(s.as_str());
              assert_eq!(interner.resolve(id), s.as_str());

              let prev = first_seen.entry(s.clone()).or_insert(id);
              assert_eq!(id, *prev);
          }

          let unique: HashSet<_> = strings.iter().collect();
          assert_eq!(interner.len(), unique.len());
      }
    }
}
