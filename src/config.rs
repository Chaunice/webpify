use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::{CompressionMode, ReplaceInputMode, ReportFormat};

/// Main configuration structure loaded from config files
#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: Option<GeneralConfig>,
    pub compression: Option<CompressionConfig>,
    pub filtering: Option<FilteringConfig>,
    pub output: Option<OutputConfig>,
    pub profiles: Option<HashMap<String, ProfileConfig>>,
}

/// Configuration profile for predefined settings
#[derive(Debug, Deserialize, Clone)]
pub struct ProfileConfig {
    pub description: Option<String>,
    pub quality: Option<u8>,
    pub mode: Option<String>,
    pub max_size: Option<u64>,
    pub preserve_structure: Option<bool>,
    pub formats: Option<Vec<String>>,
    pub threads: Option<usize>,
}

/// General configuration options
#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub input_dir: Option<String>,
    pub output_dir: Option<String>,
    pub preserve_structure: Option<bool>,
    pub overwrite: Option<bool>,
    pub threads: Option<usize>,
    pub prescan: Option<bool>,
    pub replace_input: Option<String>,
    pub reencode_webp: Option<bool>,
    pub dry_run: Option<bool>,
}

/// Compression-related configuration
#[derive(Debug, Deserialize)]
pub struct CompressionConfig {
    pub quality: Option<u8>,
    pub mode: Option<String>,
}

/// File filtering configuration
#[derive(Debug, Deserialize)]
pub struct FilteringConfig {
    pub formats: Option<Vec<String>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

/// Output and reporting configuration
#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub generate_report: Option<bool>,
    pub report_format: Option<String>,
}

/// Conversion options that can be passed to the core library
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub input_dir: PathBuf,
    pub output_dir: Option<PathBuf>,
    pub quality: u8,
    pub mode: CompressionMode,
    pub threads: Option<usize>,
    pub formats: Vec<String>,
    pub overwrite: bool,
    pub preserve_structure: bool,
    pub max_size: Option<u64>,
    pub min_size: u64,
    pub prescan: bool,
    pub replace_input: ReplaceInputMode,
    pub reencode_webp: bool,
    pub dry_run: bool,
    pub generate_report: bool,
    pub report_format: ReportFormat,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::new(),
            output_dir: None,
            quality: 80,
            mode: CompressionMode::Lossless,
            threads: None,
            formats: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "bmp".to_string(),
                "tiff".to_string(),
                "webp".to_string(),
            ],
            overwrite: false,
            preserve_structure: true,
            max_size: None,
            min_size: 1,
            prescan: true,
            replace_input: ReplaceInputMode::Off,
            reencode_webp: false,
            dry_run: false,
            generate_report: false,
            report_format: ReportFormat::Json,
        }
    }
}

impl ConversionOptions {
    /// Create new conversion options with sensible defaults
    pub fn new(input_dir: PathBuf) -> Self {
        Self {
            input_dir,
            ..Default::default()
        }
    }

    /// Builder pattern for setting quality
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }

    /// Builder pattern for setting compression mode
    pub fn with_mode(mut self, mode: CompressionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Builder pattern for setting output directory
    pub fn with_output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = Some(output_dir);
        self
    }

    /// Builder pattern for setting thread count
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Builder pattern for enabling dry run mode
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Builder pattern for setting overwrite behavior
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Builder pattern for setting preserve structure
    pub fn with_preserve_structure(mut self, preserve_structure: bool) -> Self {
        self.preserve_structure = preserve_structure;
        self
    }

    /// Builder pattern for setting minimum file size in KB
    pub fn with_min_size_kb(mut self, min_size: u64) -> Self {
        self.min_size = min_size;
        self
    }

    /// Builder pattern for setting maximum file size in MB
    pub fn with_max_size_mb(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Builder pattern for enabling prescan
    pub fn with_prescan(mut self, prescan: bool) -> Self {
        self.prescan = prescan;
        self
    }

    /// Builder pattern for setting reencode WebP behavior
    pub fn with_reencode_webp(mut self, reencode_webp: bool) -> Self {
        self.reencode_webp = reencode_webp;
        self
    }

    /// Builder pattern for setting replace input mode
    pub fn with_replace_input_mode(mut self, replace_input: ReplaceInputMode) -> Self {
        self.replace_input = replace_input;
        self
    }

    /// Builder pattern for setting supported formats
    pub fn with_supported_formats(mut self, formats: Vec<String>) -> Self {
        self.formats = formats;
        self
    }

    /// Get the effective output directory (calculated if not set)
    pub fn get_output_dir(&self) -> PathBuf {
        self.output_dir
            .clone()
            .unwrap_or_else(|| self.input_dir.join("webp_output"))
    }

    /// Get the effective thread count (calculated if not set)
    pub fn get_thread_count(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }
}
