use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ConversionStats {
    pub processed_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub skipped_count: Arc<AtomicU64>,
    #[allow(dead_code)]
    pub retry_count: Arc<AtomicU64>,
    pub original_size: Arc<AtomicU64>,
    pub compressed_size: Arc<AtomicU64>,
    format_stats: Arc<Mutex<HashMap<String, u64>>>,
    errors: Arc<Mutex<Vec<ErrorRecord>>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub file_path: String,
    pub error_message: String,
    pub retry_count: u32,
    pub timestamp: std::time::SystemTime,
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
        }
    }

    pub fn record_success(&self, original_size: u64, compressed_size: u64) {
        self.processed_count.fetch_add(1, Ordering::Relaxed);
        self.original_size.fetch_add(original_size, Ordering::Relaxed);
        self.compressed_size.fetch_add(compressed_size, Ordering::Relaxed);
    }

    pub fn record_error(&self, file_path: String, error: String) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut errors) = self.errors.lock() {
            errors.push(ErrorRecord {
                file_path,
                error_message: error,
                retry_count: 0,
                timestamp: std::time::SystemTime::now(),
            });
        }
    }

    #[allow(dead_code)]
    pub fn record_retry(&self, file_path: &str) {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut errors) = self.errors.lock() {
            // Update retry count for the most recent error for this file
            if let Some(error_record) = errors.iter_mut()
                .rev()
                .find(|e| e.file_path == file_path) {
                error_record.retry_count += 1;
            }
        }
    }

    #[allow(dead_code)]
    pub fn record_skip(&self) {
        self.skipped_count.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_format(&self, format: String) {
        if let Ok(mut stats) = self.format_stats.lock() {
            *stats.entry(format).or_insert(0) += 1;
        }
    }

    pub fn get_compression_ratio(&self) -> f64 {
        let original = self.original_size.load(Ordering::Relaxed) as f64;
        let compressed = self.compressed_size.load(Ordering::Relaxed) as f64;
        
        if original > 0.0 {
            compressed / original
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn get_space_saved(&self) -> u64 {
        let original = self.original_size.load(Ordering::Relaxed);
        let compressed = self.compressed_size.load(Ordering::Relaxed);
        original.saturating_sub(compressed)
    }

    pub fn get_format_stats(&self) -> HashMap<String, u64> {
        self.format_stats.lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    pub fn get_errors(&self) -> Vec<String> {
        self.errors.lock()
            .map(|errors| errors.iter().map(|e| format!("{}: {}", e.file_path, e.error_message)).collect())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn get_error_records(&self) -> Vec<ErrorRecord> {
        self.errors.lock()
            .map(|errors| errors.clone())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn get_success_rate(&self) -> f64 {
        let total = self.processed_count.load(Ordering::Relaxed) + self.error_count.load(Ordering::Relaxed);
        let success = self.processed_count.load(Ordering::Relaxed);
        
        if total > 0 {
            success as f64 / total as f64
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.processed_count.load(Ordering::Relaxed) == 0 && 
        self.error_count.load(Ordering::Relaxed) == 0
    }

    #[allow(dead_code)]
    pub fn total_files(&self) -> u64 {
        self.processed_count.load(Ordering::Relaxed) + 
        self.error_count.load(Ordering::Relaxed) + 
        self.skipped_count.load(Ordering::Relaxed)
    }
}

impl Default for ConversionStats {
    fn default() -> Self {
        Self::new()
    }
}
