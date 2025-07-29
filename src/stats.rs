use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ConversionStats {
    pub processed_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub skipped_count: Arc<AtomicU64>,

    pub retry_count: Arc<AtomicU64>,
    pub original_size: Arc<AtomicU64>,
    pub compressed_size: Arc<AtomicU64>,
    format_stats: Arc<Mutex<HashMap<String, u64>>>,
    errors: Arc<Mutex<Vec<ErrorRecord>>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub file_path: String,
    pub error_message: String,
    pub retry_count: u32,
}

impl Default for ConversionStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversionStats {
    pub fn new() -> Self {
        Self {
            processed_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            skipped_count: Arc::new(AtomicU64::new(0)),
            retry_count: Arc::new(AtomicU64::new(0)),
            original_size: Arc::new(AtomicU64::new(0)),
            compressed_size: Arc::new(AtomicU64::new(0)),
            format_stats: Arc::new(Mutex::new(HashMap::new())),
            errors: Arc::new(Mutex::new(Vec::new())),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start_timer(&self) {
        if let Ok(mut start_time) = self.start_time.lock() {
            *start_time = Some(Instant::now());
        }
    }

    pub fn estimate_eta(&self, total_files: u64) -> Option<std::time::Duration> {
        let processed = self.processed_count.load(Ordering::Relaxed);

        if processed == 0 || total_files == 0 {
            return None;
        }

        if let Ok(start_time) = self.start_time.lock() {
            if let Some(start) = *start_time {
                let elapsed = start.elapsed();
                let rate = processed as f64 / elapsed.as_secs_f64();
                let remaining_files = total_files.saturating_sub(processed);

                if rate > 0.0 {
                    let eta_seconds = remaining_files as f64 / rate;
                    return Some(std::time::Duration::from_secs_f64(eta_seconds));
                }
            }
        }

        None
    }

    pub fn record_success(&self, original_size: u64, compressed_size: u64) {
        self.processed_count.fetch_add(1, Ordering::Relaxed);
        self.original_size
            .fetch_add(original_size, Ordering::Relaxed);
        self.compressed_size
            .fetch_add(compressed_size, Ordering::Relaxed);
    }

    pub fn record_error(&self, file_path: String, error: String) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut errors) = self.errors.lock() {
            errors.push(ErrorRecord {
                file_path,
                error_message: error,
                retry_count: 0,
            });
        }
    }

    pub fn record_retry(&self, file_path: &str) {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut errors) = self.errors.lock() {
            // Update retry count for the most recent error for this file
            if let Some(error_record) = errors.iter_mut().rev().find(|e| e.file_path == file_path) {
                error_record.retry_count += 1;
            }
        }
    }

    pub fn record_skip(&self) {
        self.skipped_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_format(&self, format: &str) {
        if let Ok(mut format_stats) = self.format_stats.lock() {
            *format_stats.entry(format.to_string()).or_insert(0) += 1;
        }
    }

    pub fn get_compression_ratio(&self) -> f64 {
        let original = self.original_size.load(Ordering::Relaxed);
        let compressed = self.compressed_size.load(Ordering::Relaxed);

        if original == 0 {
            0.0
        } else {
            1.0 - (compressed as f64 / original as f64)
        }
    }

    pub fn get_format_stats(&self) -> std::collections::HashMap<String, u64> {
        self.format_stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_else(|_| std::collections::HashMap::new())
    }

    pub fn get_errors(&self) -> Vec<String> {
        if let Ok(errors) = self.errors.lock() {
            errors
                .iter()
                .map(|e| format!("{}: {}", e.file_path, e.error_message))
                .collect()
        } else {
            Vec::new()
        }
    }
}
