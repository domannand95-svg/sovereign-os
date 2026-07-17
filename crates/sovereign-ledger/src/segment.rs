//! Low-level storage segment handles for the file-backed append-only ledger.

use crate::{LedgerError, Lsn};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtomicPublishStage {
    PendingSynced,
    Published,
    Durable,
}

static NEXT_PENDING_ID: AtomicU64 = AtomicU64::new(0);

/// Physical lifecycle wrapper for one ledger segment file.
#[derive(Debug)]
pub struct LedgerSegment {
    file: File,
    base_lsn: Lsn,
    current_size: usize,
    max_size: usize,
    is_read_only: bool,
}

impl LedgerSegment {
    /// Creates a new appendable segment file.
    pub fn create<P: AsRef<Path>>(
        directory: P,
        base_lsn: Lsn,
        max_size: usize,
    ) -> Result<Self, LedgerError> {
        let filename = format!("{:016x}.seg", base_lsn.get());
        let path = directory.as_ref().join(filename);

        if path.exists() {
            return Err(LedgerError::WriteViolation);
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| LedgerError::WriteViolation)?;

        Ok(Self {
            file,
            base_lsn,
            current_size: 0,
            max_size,
            is_read_only: false,
        })
    }

    /// Publishes one complete record through a synchronized pending file and atomic rename.
    pub(crate) fn publish_atomic_record<P, F>(
        directory: P,
        base_lsn: Lsn,
        max_size: usize,
        bytes: &[u8],
        mut observer: F,
    ) -> Result<(), LedgerError>
    where
        P: AsRef<Path>,
        F: FnMut(AtomicPublishStage),
    {
        if bytes.is_empty() || bytes.len() > max_size {
            return Err(LedgerError::StorageExhausted);
        }

        let directory = directory.as_ref();
        let canonical = directory.join(format!("{:016x}.seg", base_lsn.get()));
        if canonical.exists() {
            return Err(LedgerError::WriteViolation);
        }

        let (pending_path, mut pending) = loop {
            let id = NEXT_PENDING_ID.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!(
                ".{:016x}.{}.{}.pending",
                base_lsn.get(),
                std::process::id(),
                id
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => break (path, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(LedgerError::WriteViolation),
            }
        };

        let result = (|| {
            pending
                .write_all(bytes)
                .map_err(|_| LedgerError::StorageExhausted)?;
            pending
                .sync_all()
                .map_err(|_| LedgerError::WriteViolation)?;
            observer(AtomicPublishStage::PendingSynced);
            drop(pending);

            // Publish without replacement. A prior exists() check cannot protect against
            // concurrent writers because ordinary rename replaces the destination on Unix.
            // Linking the already-synchronized inode fails atomically if canonical exists.
            std::fs::hard_link(&pending_path, &canonical)
                .map_err(|_| LedgerError::WriteViolation)?;
            // Canonical now owns the durable inode; stale pending cleanup is best-effort.
            let _ = std::fs::remove_file(&pending_path);
            observer(AtomicPublishStage::Published);

            File::open(directory)
                .and_then(|handle| handle.sync_all())
                .map_err(|_| LedgerError::WriteViolation)?;
            observer(AtomicPublishStage::Durable);
            Ok(())
        })();

        if result.is_err() {
            let _ = std::fs::remove_file(&pending_path);
        }
        result
    }

    /// Opens an existing segment as read-only.
    pub fn open_read_only<P: AsRef<Path>>(path: P, base_lsn: Lsn) -> Result<Self, LedgerError> {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(path)
            .map_err(|_| LedgerError::SegmentCorrupted)?;

        let metadata = file.metadata().map_err(|_| LedgerError::SegmentCorrupted)?;
        let current_size = metadata.len() as usize;

        Ok(Self {
            file,
            base_lsn,
            current_size,
            max_size: current_size,
            is_read_only: true,
        })
    }

    /// Writes raw bytes at the current segment offset.
    pub fn write_raw(&mut self, bytes: &[u8]) -> Result<(), LedgerError> {
        if self.is_read_only {
            return Err(LedgerError::WriteViolation);
        }

        let prospective_size = self
            .current_size
            .checked_add(bytes.len())
            .ok_or(LedgerError::StorageExhausted)?;

        if prospective_size > self.max_size {
            return Err(LedgerError::StorageExhausted);
        }

        self.file
            .seek(SeekFrom::Start(self.current_size as u64))
            .map_err(|_| LedgerError::WriteViolation)?;

        self.file
            .write_all(bytes)
            .map_err(|_| LedgerError::StorageExhausted)?;

        self.current_size = prospective_size;
        Ok(())
    }

    /// Reads a fixed byte window from the segment.
    pub fn read_raw(&self, offset: usize, buffer: &mut [u8]) -> Result<(), LedgerError> {
        let end_offset = offset
            .checked_add(buffer.len())
            .ok_or(LedgerError::SegmentCorrupted)?;

        if end_offset > self.current_size {
            return Err(LedgerError::SegmentCorrupted);
        }

        let mut file_handle = &self.file;

        file_handle
            .seek(SeekFrom::Start(offset as u64))
            .map_err(|_| LedgerError::SegmentCorrupted)?;

        file_handle
            .read_exact(buffer)
            .map_err(|_| LedgerError::SegmentCorrupted)?;

        Ok(())
    }

    /// Flushes buffered writes to durable storage.
    pub fn synchronize(&self) -> Result<(), LedgerError> {
        if self.is_read_only {
            return Ok(());
        }

        self.file
            .sync_all()
            .map_err(|_| LedgerError::WriteViolation)
    }

    /// Marks the segment handle as immutable after syncing.
    pub fn freeze(&mut self) -> Result<(), LedgerError> {
        self.synchronize()?;
        self.is_read_only = true;
        Ok(())
    }

    #[must_use]
    pub const fn base_lsn(&self) -> Lsn {
        self.base_lsn
    }

    #[must_use]
    pub const fn current_size(&self) -> usize {
        self.current_size
    }

    #[must_use]
    pub const fn max_size(&self) -> usize {
        self.max_size
    }

    #[must_use]
    pub const fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    #[must_use]
    pub fn has_capacity(&self, incoming_bytes_len: usize) -> bool {
        match self.current_size.checked_add(incoming_bytes_len) {
            Some(prospective_size) => prospective_size <= self.max_size,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sovereign_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn segment_creation_lifecycle() {
        let temp_dir = test_dir("seg_lifecycle");

        let base_lsn = Lsn(0x400);
        let max_size = 1024;

        let mut segment = LedgerSegment::create(&temp_dir, base_lsn, max_size).unwrap();

        assert_eq!(segment.base_lsn(), base_lsn);
        assert_eq!(segment.current_size(), 0);
        assert_eq!(segment.max_size(), max_size);
        assert!(!segment.is_read_only());
        assert!(segment.has_capacity(512));

        let data = [0xA5; 256];
        segment.write_raw(&data).unwrap();

        assert_eq!(segment.current_size(), 256);

        let mut read_buf = [0; 256];
        segment.read_raw(0, &mut read_buf).unwrap();

        assert_eq!(read_buf, data);

        drop(segment);
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn segment_exhaustion_boundaries() {
        let temp_dir = test_dir("seg_exhaustion");

        let mut segment = LedgerSegment::create(&temp_dir, Lsn(0), 100).unwrap();
        let large_payload = [0xFF; 101];

        assert_eq!(
            segment.write_raw(&large_payload),
            Err(LedgerError::StorageExhausted)
        );

        drop(segment);
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn frozen_segments_reject_mutations() {
        let temp_dir = test_dir("seg_freeze");

        let mut segment = LedgerSegment::create(&temp_dir, Lsn(1), 1024).unwrap();

        segment.write_raw(&[0x01, 0x02, 0x03]).unwrap();
        segment.freeze().unwrap();

        assert!(segment.is_read_only());
        assert_eq!(segment.write_raw(&[0x04]), Err(LedgerError::WriteViolation));

        drop(segment);
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn read_beyond_current_size_fails() {
        let temp_dir = test_dir("seg_read_bounds");

        let mut segment = LedgerSegment::create(&temp_dir, Lsn(2), 1024).unwrap();
        segment.write_raw(&[1, 2, 3, 4]).unwrap();

        let mut buffer = [0_u8; 8];

        assert_eq!(
            segment.read_raw(0, &mut buffer),
            Err(LedgerError::SegmentCorrupted)
        );

        drop(segment);
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
