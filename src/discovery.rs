//! File discovery — the one place that decides what counts as a convertable file.
//!
//! Both frontends (CLI core, GUI preview) share this rule set so a preview can
//! never advertise files the conversion run will skip. The GUI only adds
//! presentation caps (`max_files`, `max_depth`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::ConversionOptions;
use crate::utils::is_valid_image_file;

/// A discovered image file.
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub format: String,
}

/// Filters applied during discovery.
pub struct DiscoveryOptions {
    pub formats: Vec<String>,
    pub min_size_kb: u64,
    pub max_size_mb: Option<u64>,
    pub reencode_webp: bool,
    /// Stop after this many files (GUI preview).
    pub max_files: Option<usize>,
    /// Do not descend deeper than this many directory levels (GUI preview).
    pub max_depth: Option<usize>,
}

impl From<&ConversionOptions> for DiscoveryOptions {
    fn from(options: &ConversionOptions) -> Self {
        Self {
            formats: options.formats.clone(),
            min_size_kb: options.min_size,
            max_size_mb: options.max_size,
            reencode_webp: options.reencode_webp,
            max_files: None,
            max_depth: None,
        }
    }
}

pub fn discover_files(input_dir: &Path, opts: &DiscoveryOptions) -> Result<Vec<FileInfo>> {
    let mut walk = WalkDir::new(input_dir).follow_links(false);
    if let Some(max_depth) = opts.max_depth {
        walk = walk.max_depth(max_depth);
    }

    let mut files = Vec::new();
    for entry in walk {
        let entry = entry.with_context(|| {
            format!(
                "Failed to read directory entry under {}",
                input_dir.display()
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let Some(format) = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
        else {
            continue;
        };

        // Extension + webp rule first (cheap), then size (no open), then
        // header validation (only opens files that will actually convert).
        if !opts.formats.contains(&format) || (format == "webp" && !opts.reencode_webp) {
            continue;
        }

        let Ok(metadata) = std::fs::metadata(path) else {
            continue;
        };
        let size = metadata.len();
        if size < opts.min_size_kb * 1024 {
            continue;
        }
        if let Some(max_size_mb) = opts.max_size_mb
            && size > max_size_mb * 1024 * 1024
        {
            continue;
        }

        if !is_valid_image_file(path) {
            continue;
        }

        files.push(FileInfo {
            path: path.to_path_buf(),
            size,
            format,
        });
        if let Some(cap) = opts.max_files
            && files.len() >= cap
        {
            break;
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_fake_image(dir: &Path, name: &str, signature: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut data = signature.to_vec();
        data.resize(signature.len().max(2048), 0u8); // ≥2KB so default min_size passes
        fs::write(&path, data).unwrap();
        path
    }

    fn opts() -> DiscoveryOptions {
        DiscoveryOptions {
            formats: vec!["jpg".to_string(), "png".to_string(), "webp".to_string()],
            min_size_kb: 1,
            max_size_mb: None,
            reencode_webp: false,
            max_files: None,
            max_depth: None,
        }
    }

    #[test]
    fn discovers_only_valid_supported_files() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_image(dir.path(), "a.jpg", &[0xFF, 0xD8]);
        write_fake_image(
            dir.path(),
            "b.png",
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        );
        write_fake_image(dir.path(), "fake.jpg", &[0x00, 0x11]); // bad header
        write_fake_image(dir.path(), "c.txt", b"hello"); // unsupported ext
        write_fake_image(dir.path(), "e.webp", b"RIFF\x00\x00\x00\x00WEBP"); // skipped by default
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        write_fake_image(&dir.path().join("nested"), "d.jpg", &[0xFF, 0xD8]);

        let files = discover_files(dir.path(), &opts()).unwrap();
        let mut names: Vec<String> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["a.jpg", "b.png", "d.jpg"]);
    }

    #[test]
    fn reencode_webp_and_size_filters() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_image(dir.path(), "a.jpg", &[0xFF, 0xD8]);
        write_fake_image(dir.path(), "e.webp", b"RIFF\x00\x00\x00\x00WEBP");

        let mut o = opts();
        o.reencode_webp = true;
        assert_eq!(discover_files(dir.path(), &o).unwrap().len(), 2);

        // min_size in KB
        let mut o = opts();
        o.min_size_kb = 100; // files are 2KB
        assert!(discover_files(dir.path(), &o).unwrap().is_empty());
    }

    #[test]
    fn respects_caps() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            write_fake_image(dir.path(), &format!("f{i}.jpg"), &[0xFF, 0xD8]);
        }
        fs::create_dir_all(dir.path().join("deep")).unwrap();
        write_fake_image(&dir.path().join("deep"), "g.jpg", &[0xFF, 0xD8]);

        let mut o = opts();
        o.max_files = Some(3);
        assert_eq!(discover_files(dir.path(), &o).unwrap().len(), 3);

        let mut o = opts();
        o.max_depth = Some(1);
        let files = discover_files(dir.path(), &o).unwrap();
        assert!(
            files.iter().all(|f| f.path.parent().unwrap() == dir.path()),
            "max_depth must exclude nested files"
        );
    }
}
