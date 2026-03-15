/// A byte slice range `[start, start + size)` into a source string.
///
/// ```
/// # use aes_foundation::Span;
/// let text = "Aegis schema language";
/// let span = Span::sized(6, 6);
/// assert_eq!(&text[span], "schema");
/// ```
///
/// Spans use `u32` for offsets, meaning only files up to 4GB are supported, which should
/// be sufficient for reasonable programs. This decision cuts the size of `Span` in half,
/// offering a performance improvement and memory size reduction.
///
/// ## Constructing Spans
/// Span offers several constructors; In general, [`Span::sized`] is sufficient for most cases.
/// If you want to create a span from a start and end offset, you can use [`Span::from_range`].
#[derive(Debug, Clone, Copy)]
pub struct Span {
    start: u32,
    size: u32,
}

impl Span {
    #[inline]
    pub const fn sized(start: u32, size: u32) -> Self {
        Self { start, size }
    }

    /// Creates a new [`Span`] from a start and end position.
    ///
    /// # Example
    /// ```
    /// # use aes_foundation::Span;
    /// let sized = Span::sized(1, 4);
    /// let span = Span::from_range(1, 5);
    /// assert_eq!(sized.size(), span.size());
    /// ```
    ///
    /// # Invariants
    /// The `start` position must be less than or equal to `end`. Note that this
    /// invariant is checked in debug builds to avoid performance overhead.
    ///
    /// ```should_panic
    /// # use aes_foundation::Span;
    /// let span = Span::from_range(1, 0);
    /// ```
    #[inline]
    pub const fn from_range(start: u32, end: u32) -> Self {
        assert!(start <= end);
        Self {
            start,
            size: end - start,
        }
    }

    /// Creates a new empty [`Span`] of size 0.
    ///
    /// # Example
    /// ```
    /// # use aes_foundation::Span;
    /// let span = Span::empty(1);
    /// assert_eq!(span.size(), 0);
    /// ```
    #[inline]
    pub const fn empty(start: u32) -> Self {
        Self { start, size: 0 }
    }

    /// The zero-based start offset of the span
    pub const fn start(&self) -> u32 {
        self.start
    }

    /// The zero-based end offset of the span. May be equal to [`start`](Span::start()) if
    /// the span is empty, but should not be less than it.
    pub const fn end(&self) -> u32 {
        self.start + self.size
    }

    /// Get the number of bytes within the [`Span`].
    ///
    /// # Example
    /// ```
    /// # use aes_foundation::Span;
    /// assert_eq!(Span::from_range(1, 5).size(), 4);
    /// assert_eq!(Span::sized(1, 3).size(), 3);
    /// assert_eq!(Span::empty(2).size(), 0);
    /// ```
    pub const fn size(&self) -> u32 {
        self.size
    }

    /// Returns `true` if [`size`](Span::size()) is 0.
    ///
    /// # Example
    /// ```
    /// # use aes_foundation::Span;
    /// assert!(Span::empty(1).is_empty());
    /// assert!(Span::sized(1, 0).is_empty());
    /// assert!(Span::from_range(1, 1).is_empty());
    /// assert!(!Span::from_range(1, 5).is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Extracts the text slice referred to by this [`Span`] from a source string.
    ///
    /// # Panics
    ///
    /// Panics if the span is out of bounds for the given source,
    /// or if the span boundaries do not lie on UTF-8 character boundaries.
    /// Out-of-bounds spans are a compiler invariant violation and
    /// should never occur in practice.
    ///
    /// # Example
    /// ```
    /// # use aes_foundation::Span;
    /// let source = "Aegis schema language";
    /// assert_eq!(Span::sized(6, 6).text(source), "schema");
    /// ```
    pub fn text(self, source: &str) -> &str {
        debug_assert!(
            self.end() as usize <= source.len(),
            "span {self:?} out of bounds for source of length {}",
            source.len()
        );
        &source[self.start as usize..self.end() as usize]
    }

    #[must_use]
    pub fn label(self, label: impl Into<String>) -> miette::LabeledSpan {
        miette::LabeledSpan::new_with_span(Some(label.into()), self)
    }

    pub fn as_labeled(self, message: impl Into<String>) -> miette::LabeledSpan {
        miette::LabeledSpan::at(self, message)
    }
}

impl std::ops::Index<Span> for str {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self[index.start as usize..index.end() as usize]
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(s: Span) -> Self {
        (s.start() as usize, s.size() as usize).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_empty_text_returns_empty_str() {
        assert_eq!(Span::empty(1).text("hello"), "");
    }

    #[test]
    fn span_full_text_source() {
        assert_eq!(Span::sized(0, 5).text("hello"), "hello");
    }

    #[test]
    #[should_panic(expected = "not a char boundary")]
    fn span_partially_utf8_panics() {
        Span::sized(1, 1).text("héllo");
    }

    #[test]
    fn span_from_range_zero_size_is_empty() {
        let span = Span::from_range(3, 3);
        assert!(span.is_empty());
        assert_eq!(span.start(), 3);
        assert_eq!(span.end(), 3);
    }
}

#[cfg(test)]
mod size_asserts {
    use super::*;
    use crate::const_assert;

    const_assert!(size_of::<Span>() == 8);
    const_assert!(align_of::<Span>() == 4);
}
