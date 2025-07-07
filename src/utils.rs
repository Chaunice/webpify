use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate unique temporary filename
#[allow(dead_code)]
pub fn generate_temp_filename(base_name: &str, extension: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    
    format!("{}_{}.{}", base_name, timestamp, extension)
}

/// Safely create directory (if it doesn't exist)
#[allow(dead_code)]
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Get file extension (lowercase)
#[allow(dead_code)]
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Calculate relative path
#[allow(dead_code)]
pub fn get_relative_path(full_path: &Path, base_path: &Path) -> Result<PathBuf> {
    Ok(full_path.strip_prefix(base_path)?.to_path_buf())
}

/// Format file size
#[allow(dead_code)]
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let bytes_f = bytes as f64;
    let unit_index = (bytes_f.log(THRESHOLD).floor() as usize).min(UNITS.len() - 1);
    let size = bytes_f / THRESHOLD.powi(unit_index as i32);
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Validate if file is a valid image format
#[allow(dead_code)]
pub fn is_valid_image_file(path: &Path) -> bool {
    // First check extension
    let valid_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"];
    
    if let Some(ext) = get_file_extension(path) {
        if !valid_extensions.contains(&ext.as_str()) {
            return false;
        }
    } else {
        return false;
    }
    
    // Could add deeper file header checking here
    true
}

/// Calculate compression ratio between two file sizes
#[allow(dead_code)]
pub fn calculate_compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
    if original_size == 0 {
        return 0.0;
    }
    
    compressed_size as f64 / original_size as f64
}

/// Get optimal thread count for the system
#[allow(dead_code)]
pub fn get_optimal_thread_count() -> usize {
    let cpu_count = num_cpus::get();
    
    // For I/O intensive tasks, we can use more threads than CPU cores
    // But also consider memory usage
    (cpu_count * 2).min(16) // Max 16 threads
}

/// Check if there's enough disk space
#[allow(dead_code)]
pub fn check_disk_space(_path: &Path, _required_space: u64) -> Result<bool> {
    // Platform-specific disk space checking would go here
    // Simplified version always returns true
    Ok(true)
}

/// Create safe output filename (avoid path traversal attacks)
#[allow(dead_code)]
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_calculate_compression_ratio() {
        assert_eq!(calculate_compression_ratio(1000, 500), 0.5);
        assert_eq!(calculate_compression_ratio(0, 100), 0.0);
        assert_eq!(calculate_compression_ratio(1000, 1000), 1.0);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.jpg"), "test.jpg");
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_filename("file with spaces.png"), "file with spaces.png");
    }
}
