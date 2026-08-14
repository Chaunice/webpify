use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ConversionStats {
    pub processed_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub skipped_count: Arc<AtomicU64>,

    pub original_size: Arc<AtomicU64>,
    pub compressed_size: Arc<AtomicU64>,
    format_stats: Arc<Mutex<HashMap<String, u64>>>,
    errors: Arc<Mutex<Vec<ErrorRecord>>>,
}

#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub file_path: String,
    pub error_message: String,
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
            original_size: Arc::new(AtomicU64::new(0)),
            compressed_size: Arc::new(AtomicU64::new(0)),
            format_stats: Arc::new(Mutex::new(HashMap::new())),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
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
            });
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
