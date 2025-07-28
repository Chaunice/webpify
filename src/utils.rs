use std::fmt;
impl fmt::Display for ImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageValidationError::InvalidExtension => write!(f, "Invalid file extension"),
            ImageValidationError::FileNotFound => write!(f, "File not found"),
            ImageValidationError::IoError(e) => write!(f, "IO error: {}", e),
            ImageValidationError::InvalidHeader => write!(f, "Invalid image header"),
            ImageValidationError::FileTooSmall => write!(f, "File too small to be a valid image"),
        }
    }
}
use std::fs::File;
use std::io::{self, Read};
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

/// Supported image formats with their magic number signatures
const IMAGE_SIGNATURES: &[(&str, &[&[u8]])] = &[
    ("jpg", &[&[0xFF, 0xD8]]),
    ("jpeg", &[&[0xFF, 0xD8]]),
    ("png", &[&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]]),
    ("gif", &[b"GIF87a", b"GIF89a"]),
    ("bmp", &[b"BM"]),
    ("tiff", &[&[0x49, 0x49, 0x2A, 0x00], &[0x4D, 0x4D, 0x00, 0x2A]]),
    ("webp", &[]), // WebP needs special handling
];

/// Error types for image validation
#[derive(Debug)]
pub enum ImageValidationError {
    InvalidExtension,
    FileNotFound,
    IoError(io::Error),
    InvalidHeader,
    FileTooSmall,
}

impl From<io::Error> for ImageValidationError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

/// Validate if file is a valid image format with deep header checking
pub fn is_valid_image_file(path: &Path) -> bool {
    validate_image_file(path).is_ok()
}

/// Validate image file with detailed error information
pub fn validate_image_file(path: &Path) -> Result<(), ImageValidationError> {
    // Check if file exists
    if !path.exists() {
        return Err(ImageValidationError::FileNotFound);
    }

    // Check extension
    let extension = get_file_extension(path)
        .ok_or(ImageValidationError::InvalidExtension)?;
    
    if !is_supported_extension(&extension) {
        return Err(ImageValidationError::InvalidExtension);
    }

    // Validate file header
    validate_image_header(path, &extension)
}

/// Check if extension is supported
fn is_supported_extension(extension: &str) -> bool {
    IMAGE_SIGNATURES.iter()
        .any(|(ext, _)| *ext == extension)
}

/// Validate image file headers to prevent processing of corrupted or fake files
fn validate_image_header(path: &Path, extension: &str) -> Result<(), ImageValidationError> {
    let mut file = File::open(path)?;
    
    // Determine required header size
    let header_size = match extension {
        "webp" => 12,
        "png" => 8,
        "gif" => 6,
        "tiff" => 4,
        _ => 2,
    };
    
    let mut header = vec![0u8; header_size];
    let bytes_read = file.read(&mut header)?;
    
    if bytes_read < header_size {
        return Err(ImageValidationError::FileTooSmall);
    }

    let is_valid = match extension {
        "jpg" | "jpeg" => header[0] == 0xFF && header[1] == 0xD8,
        "png" => header == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "gif" => header[0..6] == *b"GIF87a" || header[0..6] == *b"GIF89a",
        "bmp" => header[0..2] == *b"BM",
        "tiff" => {
            header[0..4] == [0x49, 0x49, 0x2A, 0x00] || // little-endian
            header[0..4] == [0x4D, 0x4D, 0x00, 0x2A]    // big-endian
        },
        "webp" => {
            header[0..4] == *b"RIFF" && header[8..12] == *b"WEBP"
        },
        _ => return Err(ImageValidationError::InvalidExtension),
    };

    if is_valid {
        Ok(())
    } else {
        Err(ImageValidationError::InvalidHeader)
    }
}
