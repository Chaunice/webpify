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
            replace_input: ReplaceInputMode::Off,
            reencode_webp: false,
            dry_run: false,
            generate_report: false,
            report_format: ReportFormat::Json,
        }
    }
}

impl ConversionOptions {
    /// Get the effective output directory (calculated if not set)
    pub fn get_output_dir(&self) -> PathBuf {
        self.output_dir
            .clone()
            .unwrap_or_else(|| self.input_dir.join("webp_output"))
    }
}
