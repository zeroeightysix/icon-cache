//! Load icon caches from a file path in a safe manner

use crate::IconCache;
use file_lock::FileLock;
use memmap2::Mmap;
use std::error::Error;
use std::ops::Deref;
use std::os::fd::AsRawFd;
use std::path::Path;

/// Reexports `file_lock` and `memmap2`, which are used in the [OwnedIconCache] type.
pub mod reexports {
    pub use file_lock;
    pub use memmap2;
}

/// Provides access to an [IconCache] constructed from a file that is guaranteed not to be modified.
/// 
/// `OwnedIconCache` holds a lock on the cache file and creates a memory-mapped region with the file's
/// contents inside. It does not copy the file contents.
/// 
/// To access the icon cache, use [OwnedIconCache::icon_cache]
#[derive(Debug)]
pub struct OwnedIconCache {
    pub lock: FileLock,
    pub memmap: Mmap,
}

impl OwnedIconCache {
    /// Open and lock a file. This call may block waiting to acquire a lock if an exclusive lock
    /// is already held.
    ///
    /// If this behaviour is undesirable, use [open_non_blocking](Self::open_non_blocking) instead.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::create(path, true)
    }

    /// Open and lock a file, returning an error if an exclusive lock on the file was already held 
    /// by another process.
    pub fn open_non_blocking(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::create(path, false)
    }

    /// Access the icon cache held by this `OwnedIconCache`.
    ///
    /// Returns an error if the cache could not be parsed.
    pub fn icon_cache<'a>(&'a self) -> Result<IconCache<'a>, Box<dyn Error + 'a>> {
        let bytes = self.memmap.deref();
        IconCache::new_from_bytes(bytes)
    }

    fn create(path: impl AsRef<Path>, blocking: bool) -> std::io::Result<Self> {
        let path = path.as_ref();
        let options = file_lock::FileOptions::new().write(false); // we explicitly do NOT want to write to the cache!

        let lock = FileLock::lock(path, blocking, options)?;

        Self::from_lock(lock)
    }

    /// Create a `OwnedIconCache` from a locked file
    pub fn from_lock(lock: FileLock) -> std::io::Result<Self> {
        let fd = lock.file.as_raw_fd();
        // SAFETY: we hold `lock`, which claims that `fd` will not change (unless done by us, which we won't)
        // throughout the lifetime of the lock
        let memmap = unsafe { Mmap::map(fd)? };

        Ok(Self { lock, memmap })
    }
}
