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

/// Validate if file is a valid image format with deep header checking
pub fn is_valid_image_file(path: &Path) -> bool {
    // First check extension
    let valid_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"];

    let extension = if let Some(ext) = get_file_extension(path) {
        if !valid_extensions.contains(&ext.as_str()) {
            return false;
        }
        ext
    } else {
        return false;
    };

    // Deep validation: check file headers (magic numbers)
    validate_image_header(path, &extension)
}

/// Validate image file headers to prevent processing of corrupted or fake files
fn validate_image_header(path: &Path, extension: &str) -> bool {
    use std::fs::File;
    use std::io::Read;

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut header = [0u8; 16]; // Read first 16 bytes
    if file.read(&mut header).unwrap_or(0) < 4 {
        return false; // File too small to be a valid image
    }

    match extension {
        "jpg" | "jpeg" => {
            // JPEG files start with FF D8 and end with FF D9
            header[0] == 0xFF && header[1] == 0xD8
        }
        "png" => {
            // PNG signature: 89 50 4E 47 0D 0A 1A 0A
            header[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        }
        "gif" => {
            // GIF signature: "GIF87a" or "GIF89a"
            &header[0..6] == b"GIF87a" || &header[0..6] == b"GIF89a"
        }
        "bmp" => {
            // BMP signature: "BM"
            &header[0..2] == b"BM"
        }
        "tiff" => {
            // TIFF signatures: "II*\0" (little-endian) or "MM\0*" (big-endian)
            (&header[0..4] == [0x49, 0x49, 0x2A, 0x00])
                || (&header[0..4] == [0x4D, 0x4D, 0x00, 0x2A])
        }
        "webp" => {
            // WebP signature: "RIFF" at start and "WEBP" at offset 8
            &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP"
        }
        _ => true, // For unknown extensions, assume valid
    }
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
pub fn check_disk_space(path: &Path, required_space: u64) -> Result<bool> {
    // Cross-platform disk space checking
    use std::fs;

    if let Ok(metadata) = fs::metadata(path) {
        if metadata.is_dir() {
            // For directories, check the parent filesystem
            return check_filesystem_space(path, required_space);
        }
    }

    // For files, check the parent directory
    if let Some(parent) = path.parent() {
        check_filesystem_space(parent, required_space)
    } else {
        // Fallback: assume sufficient space
        Ok(true)
    }
}

#[cfg(windows)]
fn check_filesystem_space(path: &Path, required_space: u64) -> Result<bool> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    let path_wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;

    unsafe {
        let result = windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path_wide.as_ptr(),
            &mut free_bytes,
            &mut total_bytes,
            ptr::null_mut(),
        );

        if result != 0 {
            Ok(free_bytes >= required_space)
        } else {
            // If we can't get disk space, assume it's available
            Ok(true)
        }
    }
}

#[cfg(unix)]
fn check_filesystem_space(path: &Path, required_space: u64) -> Result<bool> {
    use std::ffi::CString;
    use std::mem;

    let path_cstr = CString::new(path.to_string_lossy().as_bytes())?;

    unsafe {
        let mut statvfs: libc::statvfs = mem::zeroed();
        let result = libc::statvfs(path_cstr.as_ptr(), &mut statvfs);

        if result == 0 {
            let available_bytes = statvfs.f_bavail * statvfs.f_frsize;
            Ok(available_bytes >= required_space)
        } else {
            // If we can't get disk space, assume it's available
            Ok(true)
        }
    }
}

#[cfg(not(any(windows, unix)))]
fn check_filesystem_space(_path: &Path, _required_space: u64) -> Result<bool> {
    // For other platforms, assume space is available
    Ok(true)
}

/// Format duration for human-readable display
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
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
        assert_eq!(
            sanitize_filename("file with spaces.png"),
            "file with spaces.png"
        );
    }
}
