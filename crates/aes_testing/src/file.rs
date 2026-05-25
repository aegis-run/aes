use std::path::Path;

use aes_allocator::Allocator;
use aes_foundation::vfs::{FileId, FileRef};

/// Sentinel [`FileId`] used when constructing a [`FileRef`] outside a [`Vfs`] (e.g. in tests).
pub const ANONYMOUS_FILE_ID: FileId = FileId::new(u32::MAX);

/// Creates a [`FileRef`] from a borrowed allocator and source string.
///
/// This avoids spinning up a full [`Vfs`] in unit tests. The file gets a
/// sentinel [`ANONYMOUS_FILE_ID`].
pub fn file_ref<'a>(alloc: &'a Allocator, source: &'a str) -> FileRef<'a> {
    FileRef::new(ANONYMOUS_FILE_ID, Path::new("anonymous.aes"), alloc, source)
}
