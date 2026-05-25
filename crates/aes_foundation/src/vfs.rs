use std::path::{Path, PathBuf};

use aes_allocator::Allocator;
use rustc_hash::FxHashMap;

use crate::Id;

/// An in-memory representation of a source file and its associated resources.
pub struct File {
    path: PathBuf,
    source: String,
    alloc: aes_allocator::Allocator,
}

/// A unique identifier for a file managed by the [`Vfs`].
pub type FileId = Id<File>;

/// A lightweight, copyable reference to a file's resources.
///
/// `FileRef` provides access to the file's source code and its dedicated
/// memory arena without owning the underlying data.
#[derive(Debug, Clone, Copy)]
pub struct FileRef<'a> {
    id: FileId,
    path: &'a Path,
    alloc: &'a aes_allocator::Allocator,
    source: &'a str,
}

impl<'a> FileRef<'a> {
    pub fn new(id: FileId, path: &'a Path, alloc: &'a Allocator, source: &'a str) -> Self {
        Self {
            id,
            path,
            alloc,
            source,
        }
    }

    /// Returns the unique identifier for this file.
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Returns the path to the file.
    pub fn path(&self) -> &'a Path {
        self.path
    }

    /// Returns the source code of the file.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Returns the memory allocator dedicated to this file.
    pub fn alloc(&self) -> &'a aes_allocator::Allocator {
        self.alloc
    }
}

/// A Virtual File System that manages source files and their lifetimes.
///
/// The `Vfs` is the owner of all source strings and the allocators used
/// during the compilation of each file. It issues [`FileId`]s which can
/// be used to retrieve a [`FileRef`] for processing.
#[derive(Default)]
pub struct Vfs {
    files: FxHashMap<FileId, File>,
    next_id: u32,
}

impl Vfs {
    #[inline(always)]
    fn gen_id(&mut self) -> FileId {
        let ret = FileId::new(self.next_id);
        self.next_id += 1;
        ret
    }

    pub fn add(&mut self, path: &Path, source: impl Into<String>) -> FileId {
        let id = self.gen_id();

        self.files.insert(
            id,
            File {
                path: path.into(),
                source: source.into(),
                alloc: Allocator::new(),
            },
        );

        id
    }

    pub fn get(&self, id: FileId) -> Option<FileRef<'_>> {
        let file = self.files.get(&id)?;
        Some(FileRef::new(id, &file.path, &file.alloc, &file.source))
    }
}
