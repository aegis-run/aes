use std::marker::PhantomData;

#[repr(transparent)]
pub struct Id<T> {
    index: u32,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn as_index(&self) -> usize {
        self.index as usize
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Id").field("index", &self.index).finish()
    }
}

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

pub struct Range<T> {
    start: Id<T>,
    end: Id<T>,
}

impl<T> Range<T> {
    #[inline]
    pub fn new(start: Id<T>, end: Id<T>) -> Self {
        debug_assert!(start.index <= end.index);
        Self { start, end }
    }

    #[inline]
    pub fn empty(at: Id<T>) -> Self {
        Self::new(at, at)
    }

    #[inline]
    pub const fn start(&self) -> Id<T> {
        self.start
    }

    #[inline]
    pub const fn end(&self) -> Id<T> {
        self.end
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.end.index as usize - self.start.index as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start.index == self.end.index
    }

    #[inline]
    pub fn iter(self) -> impl Iterator<Item = Id<T>> {
        (self.start.index..self.end.index).map(Id::new)
    }
}

impl<T> Copy for Range<T> {}
impl<T> Clone for Range<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for Range<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Range")
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

pub struct Checkpoint<T>(Id<T>);

impl<T> Checkpoint<T> {
    #[inline]
    pub const fn new(val: Id<T>) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn id(&self) -> Id<T> {
        self.0
    }
}

impl<T> Copy for Checkpoint<T> {}
impl<T> Clone for Checkpoint<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for Checkpoint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Checkpoint").field(&self.0).finish()
    }
}
