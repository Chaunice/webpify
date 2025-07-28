use std::path::Path;
use std::time::Duration;

/// Format duration in human-readable format
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Get file extension (lowercase)
fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
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
            // JPEG files start with FF D8
            header.len() >= 2 && header[0] == 0xFF && header[1] == 0xD8
        }
        "png" => {
            // PNG signature: 89 50 4E 47 0D 0A 1A 0A
            header.len() >= 8 && header[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        }
        "gif" => {
            // GIF signature: "GIF87a" or "GIF89a"
            header.len() >= 6 && (&header[0..6] == b"GIF87a" || &header[0..6] == b"GIF89a")
        }
        "bmp" => {
            // BMP signature: "BM"
            header.len() >= 2 && &header[0..2] == b"BM"
        }
        "tiff" => {
            // TIFF signatures: "II*\0" (little-endian) or "MM\0*" (big-endian)
            header.len() >= 4 && ((&header[0..4] == [0x49, 0x49, 0x2A, 0x00])
                || (&header[0..4] == [0x4D, 0x4D, 0x00, 0x2A]))
        }
        "webp" => {
            // WebP signature: "RIFF" at start and "WEBP" at offset 8
            header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP"
        }
        _ => true, // For unknown extensions, assume valid
    }
}