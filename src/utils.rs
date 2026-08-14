use std::fmt;
impl fmt::Display for ImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageValidationError::InvalidExtension => write!(f, "Invalid file extension"),
            ImageValidationError::FileNotFound => write!(f, "File not found"),
            ImageValidationError::IoError(e) => write!(f, "IO error: {e}"),
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
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Get file extension (lowercase)
fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Supported image formats with their magic number signatures.
/// Formats with an empty signature list (or none at all) have no reliable
/// magic number and are handled case-by-case in `validate_image_header`.
const IMAGE_SIGNATURES: &[(&str, &[&[u8]])] = &[
    ("jpg", &[&[0xFF, 0xD8]]),
    ("jpeg", &[&[0xFF, 0xD8]]),
    ("png", &[&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]]),
    ("gif", &[b"GIF87a", b"GIF89a"]),
    ("bmp", &[b"BM"]),
    (
        "tiff",
        &[&[0x49, 0x49, 0x2A, 0x00], &[0x4D, 0x4D, 0x00, 0x2A]],
    ),
    ("webp", &[]), // RIFF/WEBP box — handled in validate_image_header
    ("ico", &[&[0x00, 0x00, 0x01, 0x00]]),
    ("pnm", &[b"P1", b"P2", b"P3", b"P4", b"P5", b"P6", b"P7"]),
    ("qoi", &[b"qoif"]),
    ("hdr", &[b"#?RADIANCE", b"#?RGBE"]),
    ("exr", &[&[0x76, 0x2F, 0x31, 0x01]]),
    ("dds", &[b"DDS "]),
    ("farbfeld", &[b"farbfeld"]),
    ("tga", &[]), // no reliable magic number — accept any readable file
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
    let extension = get_file_extension(path).ok_or(ImageValidationError::InvalidExtension)?;

    if !is_supported_extension(&extension) {
        return Err(ImageValidationError::InvalidExtension);
    }

    // Validate file header
    validate_image_header(path, &extension)
}

/// Check if extension is supported
fn is_supported_extension(extension: &str) -> bool {
    IMAGE_SIGNATURES.iter().any(|(ext, _)| *ext == extension)
}

fn signatures_for(extension: &str) -> Option<&'static [&'static [u8]]> {
    IMAGE_SIGNATURES
        .iter()
        .find(|(ext, _)| *ext == extension)
        .map(|(_, signatures)| *signatures)
}

/// Validate image file headers to prevent processing of corrupted or fake files
fn validate_image_header(path: &Path, extension: &str) -> Result<(), ImageValidationError> {
    let mut file = File::open(path)?;

    let mut header = [0u8; 12];
    let bytes_read = file.read(&mut header)?;
    let header = &header[..bytes_read];

    let is_valid = match extension {
        // RIFF .... WEBP
        "webp" => header.len() >= 12 && header.starts_with(b"RIFF") && &header[8..12] == b"WEBP",
        // No reliable magic number — accept any non-empty file
        "tga" => bytes_read > 0,
        _ => {
            let Some(signatures) = signatures_for(extension) else {
                return Err(ImageValidationError::InvalidExtension);
            };
            let min_len = signatures.iter().map(|sig| sig.len()).min().unwrap_or(0);
            if bytes_read < min_len {
                return Err(ImageValidationError::FileTooSmall);
            }
            signatures
                .iter()
                .any(|signature| header.starts_with(signature))
        }
    };

    if is_valid {
        Ok(())
    } else {
        Err(ImageValidationError::InvalidHeader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn validates_all_supported_formats() {
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            ("a.jpg", &[0xFF, 0xD8][..]),
            (
                "b.png",
                &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A][..],
            ),
            ("c.gif", &b"GIF89a"[..]),
            ("d.bmp", &b"BM"[..]),
            ("e.tiff", &[0x49, 0x49, 0x2A, 0x00][..]),
            ("f.ico", &[0x00, 0x00, 0x01, 0x00][..]),
            ("g.pnm", &b"P6\n1 1\n255\n"[..]),
            ("h.qoi", &b"qoif"[..]),
            ("i.hdr", &b"#?RADIANCE\n"[..]),
            ("j.exr", &[0x76, 0x2F, 0x31, 0x01][..]),
            ("k.dds", &b"DDS "[..]),
            ("l.farbfeld", &b"farbfeld"[..]),
            ("m.tga", &b"no magic here but valid enough"[..]),
            ("n.webp", &b"RIFF\x00\x00\x00\x00WEBP"[..]),
        ];
        for (name, bytes) in cases {
            let path = write_file(dir.path(), name, bytes);
            assert!(is_valid_image_file(&path), "{name} should validate");
        }
    }

    #[test]
    fn rejects_fakes_and_unknowns() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "fake.jpg", &[0x00, 0x11, 0x22, 0x33]);
        assert!(!is_valid_image_file(&path));
        let path = write_file(dir.path(), "fake.ico", &[0x01, 0x00, 0x01, 0x00]);
        assert!(!is_valid_image_file(&path));
        let path = write_file(dir.path(), "note.txt", b"P6\n");
        assert!(!is_valid_image_file(&path));
        let path = write_file(dir.path(), "empty.tga", b"");
        assert!(!is_valid_image_file(&path));
        let path = write_file(dir.path(), "tiny.png", &[0x89, 0x50]);
        assert!(!is_valid_image_file(&path));
    }
}
