//! src/services/wal_service.rs
//! Write-Ahead Log (WAL) persistence engine with binary framing, CRC32 validation, and crash recovery.
//! Connects to: src/models/wal.rs, src/models/event.rs, src/services/ring_buffer.rs
//! Created: 2026-08-28

use crc32fast::Hasher;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{IngestEvent, WalStatsResponse};

/// 4-byte magic marker for WAL binary frames: "WAL1"
const WAL_MAGIC: &[u8; 4] = b"WAL1";
const HEADER_SIZE: usize = 12; // 4 bytes magic + 4 bytes length + 4 bytes CRC32

/// Thread-safe Write-Ahead Log engine for asynchronous crash recovery and event persistence.
pub struct WalService {
    file_path: PathBuf,
    file: RwLock<File>,
    total_appends: AtomicU64,
    total_bytes_written: AtomicU64,
    corrupted_frames_skipped: AtomicU64,
    recovered_on_boot: AtomicUsize,
    last_synced_ms: AtomicU64,
}

impl WalService {
    /// Initializes a new WAL service instance, creating parent directories and opening log file.
    ///
    /// # Arguments
    /// * `path` - Destination path for WAL log file
    ///
    /// # Returns
    /// An instantiated `WalService` or I/O error.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file_path = path.as_ref().to_path_buf();
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&file_path)?;

        let initial_size = file.metadata()?.len();

        Ok(Self {
            file_path,
            file: RwLock::new(file),
            total_appends: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(initial_size),
            corrupted_frames_skipped: AtomicU64::new(0),
            recovered_on_boot: AtomicUsize::new(0),
            last_synced_ms: AtomicU64::new(Self::current_epoch_ms()),
        })
    }

    /// Current epoch timestamp in milliseconds.
    #[inline]
    fn current_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Encodes a single event into a binary WAL frame: `[MAGIC(4)][LEN(4)][CRC32(4)][PAYLOAD]`.
    fn encode_frame(event: &IngestEvent) -> std::io::Result<Vec<u8>> {
        let payload = serde_json::to_vec(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let checksum = hasher.finalize();

        let payload_len = payload.len() as u32;
        let mut frame = Vec::with_capacity(HEADER_SIZE + payload.len());

        frame.extend_from_slice(WAL_MAGIC);
        frame.extend_from_slice(&payload_len.to_le_bytes());
        frame.extend_from_slice(&checksum.to_le_bytes());
        frame.extend_from_slice(&payload);

        Ok(frame)
    }

    /// Appends a single event to the write-ahead log.
    ///
    /// # Arguments
    /// * `event` - Event data to persist
    ///
    /// # Returns
    /// Number of binary bytes written to disk.
    pub fn append_event(&self, event: &IngestEvent) -> std::io::Result<usize> {
        let frame = Self::encode_frame(event)?;
        let frame_len = frame.len();

        if let Ok(mut file_guard) = self.file.write() {
            file_guard.write_all(&frame)?;
            self.total_appends.fetch_add(1, Ordering::Relaxed);
            self.total_bytes_written
                .fetch_add(frame_len as u64, Ordering::Relaxed);
        }

        Ok(frame_len)
    }

    /// Appends a batch of events sequentially in a single contiguous write.
    ///
    /// # Arguments
    /// * `events` - Slice of event records
    ///
    /// # Returns
    /// Total binary bytes written.
    pub fn append_batch(&self, events: &[IngestEvent]) -> std::io::Result<usize> {
        let mut batch_buffer = Vec::with_capacity(events.len() * 256);

        for event in events {
            let frame = Self::encode_frame(event)?;
            batch_buffer.extend_from_slice(&frame);
        }

        let total_bytes = batch_buffer.len();

        if let Ok(mut file_guard) = self.file.write() {
            file_guard.write_all(&batch_buffer)?;
            self.total_appends
                .fetch_add(events.len() as u64, Ordering::Relaxed);
            self.total_bytes_written
                .fetch_add(total_bytes as u64, Ordering::Relaxed);
        }

        Ok(total_bytes)
    }

    /// Replays the WAL log file from beginning to end to recover uncorrupted events on boot.
    ///
    /// # Returns
    /// Vector of valid recovered `IngestEvent` records.
    pub fn recover(&self) -> std::io::Result<Vec<IngestEvent>> {
        let mut recovered_events = Vec::new();
        let file = File::open(&self.file_path)?;
        let file_len = file.metadata()?.len();

        if file_len < HEADER_SIZE as u64 {
            return Ok(recovered_events);
        }

        let mut reader = BufReader::new(file);
        let mut header_buf = [0u8; HEADER_SIZE];

        while let Ok(()) = reader.read_exact(&mut header_buf) {
            // Verify magic marker
            if &header_buf[0..4] != WAL_MAGIC {
                self.corrupted_frames_skipped.fetch_add(1, Ordering::Relaxed);
                // Attempt to scan ahead by 1 byte
                let _ = reader.seek(SeekFrom::Current(-(HEADER_SIZE as i64 - 1)));
                continue;
            }

            let payload_len = u32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;
            let expected_crc = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());

            // Sanity check length
            if payload_len > 16 * 1024 * 1024 {
                self.corrupted_frames_skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            let mut payload_buf = vec![0u8; payload_len];
            if reader.read_exact(&mut payload_buf).is_err() {
                // Incomplete frame at end of log
                self.corrupted_frames_skipped.fetch_add(1, Ordering::Relaxed);
                break;
            }

            // Verify CRC32
            let mut hasher = Hasher::new();
            hasher.update(&payload_buf);
            let actual_crc = hasher.finalize();

            if actual_crc != expected_crc {
                self.corrupted_frames_skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Deserialize JSON payload
            if let Ok(event) = serde_json::from_slice::<IngestEvent>(&payload_buf) {
                recovered_events.push(event);
            } else {
                self.corrupted_frames_skipped.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.recovered_on_boot
            .store(recovered_events.len(), Ordering::Relaxed);
        Ok(recovered_events)
    }

    /// Forces synchronous `fsync` flush of kernel page caches to physical storage.
    ///
    /// # Returns
    /// Total bytes flushed.
    pub fn sync(&self) -> std::io::Result<u64> {
        if let Ok(mut file_guard) = self.file.write() {
            file_guard.flush()?;
            file_guard.sync_all()?;
            self.last_synced_ms
                .store(Self::current_epoch_ms(), Ordering::Relaxed);
            let size = file_guard.metadata()?.len();
            Ok(size)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to acquire WAL write lock",
            ))
        }
    }

    /// Checkpoints the log by truncating the file to 0 bytes after state consolidation.
    ///
    /// # Returns
    /// File size in bytes prior to truncation.
    pub fn checkpoint(&self) -> std::io::Result<u64> {
        if let Ok(mut file_guard) = self.file.write() {
            file_guard.flush()?;
            file_guard.sync_all()?;
            let previous_size = file_guard.metadata()?.len();

            // Re-open with truncate
            drop(file_guard);
            let truncated_file = OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(true)
                .open(&self.file_path)?;

            if let Ok(mut guard) = self.file.write() {
                *guard = truncated_file;
                self.total_bytes_written.store(0, Ordering::Relaxed);
            }

            Ok(previous_size)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to acquire WAL lock for checkpoint",
            ))
        }
    }

    /// Compiles telemetry statistics for the WAL persistence service.
    pub fn get_stats(&self) -> WalStatsResponse {
        let file_size_bytes = self
            .file
            .read()
            .ok()
            .and_then(|f| f.metadata().ok().map(|m| m.len()))
            .unwrap_or(0);

        WalStatsResponse {
            file_path: self.file_path.to_string_lossy().to_string(),
            file_size_bytes,
            total_appends: self.total_appends.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            recovered_on_boot: self.recovered_on_boot.load(Ordering::Relaxed),
            corrupted_frames_skipped: self.corrupted_frames_skipped.load(Ordering::Relaxed),
            last_synced_ms: self.last_synced_ms.load(Ordering::Relaxed),
        }
    }
}
