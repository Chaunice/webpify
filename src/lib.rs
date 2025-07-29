//! # Webpify Library
//! 
//! High-performance batch image to WebP converter core library.
//! 
//! This library provides the core functionality for converting images to WebP format,
//! with support for parallel processing, different compression modes, and comprehensive
//! progress tracking.

pub mod config;
pub mod converter;
pub mod core;
pub mod progress;
pub mod stats;
pub mod utils;

// Re-export commonly used types
pub use config::{ConversionOptions, Config, ProfileConfig};
pub use converter::ImageConverter;
pub use core::WebpifyCore;
pub use progress::ProgressReporter;
pub use stats::ConversionStats;
pub use utils::{format_duration, is_valid_image_file, validate_image_file, ImageValidationError};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main conversion report structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversionReport {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub total_files: u64,
    pub processed_files: u64,
    pub failed_files: u64,
    pub skipped_files: u64,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub files_per_second: f64,
    pub bytes_per_second: u64,
    pub thread_count: usize,
    pub quality: u8,
    pub mode: String,
    pub format_stats: HashMap<String, u64>,
    pub errors: Vec<String>,
}

/// Report output formats
#[derive(Debug, Clone, PartialEq)]
pub enum ReportFormat {
    Json,
    Csv,
    Html,
}

/// Compression modes for WebP conversion
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionMode {
    /// Lossless compression (larger files but perfect quality)
    Lossless,
    /// Lossy compression (smaller files with slight quality loss)
    Lossy,
    /// Auto mode (intelligently choose based on image characteristics)
    Auto,
}

/// How to handle input files after successful conversion
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceInputMode {
    /// Do not delete input files (default)
    Off,
    /// Move input files to recycle bin after successful conversion
    Recycle,
    /// Permanently delete input files after successful conversion
    Delete,
}

/// Generate a conversion report in the specified format
pub fn generate_report(report: &ConversionReport, format: &ReportFormat) -> Result<()> {
    match format {
        ReportFormat::Json => generate_json_report(report),
        ReportFormat::Csv => generate_csv_report(report),
        ReportFormat::Html => generate_html_report(report),
    }
}

fn generate_json_report(report: &ConversionReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    let report_path = "webpify_report.json";
    std::fs::write(report_path, json)?;
    println!("Report saved to: {report_path}");
    Ok(())
}

fn generate_csv_report(report: &ConversionReport) -> Result<()> {
    use std::io::Write;
    
    let report_path = "webpify_report.csv";
    let mut file = std::fs::File::create(report_path)?;
    
    // Write CSV header
    writeln!(file, "metric,value")?;
    writeln!(file, "start_time,{}", report.start_time.format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(file, "end_time,{}", report.end_time.format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(file, "duration_seconds,{}", report.duration.as_secs())?;
    writeln!(file, "input_dir,{}", report.input_dir.display())?;
    writeln!(file, "output_dir,{}", report.output_dir.display())?;
    writeln!(file, "total_files,{}", report.total_files)?;
    writeln!(file, "processed_files,{}", report.processed_files)?;
    writeln!(file, "failed_files,{}", report.failed_files)?;
    writeln!(file, "skipped_files,{}", report.skipped_files)?;
    writeln!(file, "original_size_bytes,{}", report.original_size)?;
    writeln!(file, "compressed_size_bytes,{}", report.compressed_size)?;
    writeln!(file, "compression_ratio,{:.2}", report.compression_ratio)?;
    writeln!(file, "files_per_second,{:.2}", report.files_per_second)?;
    writeln!(file, "bytes_per_second,{}", report.bytes_per_second)?;
    writeln!(file, "thread_count,{}", report.thread_count)?;
    writeln!(file, "quality,{}", report.quality)?;
    writeln!(file, "mode,{}", report.mode)?;
    
    println!("Report saved to: {report_path}");
    Ok(())
}

fn generate_html_report(report: &ConversionReport) -> Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Webpify Conversion Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .header {{ color: #2c3e50; }}
        .stats {{ background: #f8f9fa; padding: 20px; border-radius: 5px; }}
        .metric {{ margin: 10px 0; }}
        .success {{ color: #27ae60; }}
        .error {{ color: #e74c3c; }}
    </style>
</head>
<body>
    <h1 class="header">Webpify Conversion Report</h1>
    <div class="stats">
        <div class="metric"><strong>Duration:</strong> {} seconds</div>
        <div class="metric"><strong>Files Processed:</strong> <span class="success">{}</span></div>
        <div class="metric"><strong>Files Failed:</strong> <span class="error">{}</span></div>
        <div class="metric"><strong>Files Skipped:</strong> {}</div>
        <div class="metric"><strong>Compression Ratio:</strong> {:.2}%</div>
        <div class="metric"><strong>Processing Speed:</strong> {:.2} files/sec</div>
        <div class="metric"><strong>Quality:</strong> {}</div>
        <div class="metric"><strong>Mode:</strong> {}</div>
    </div>
</body>
</html>"#,
        report.duration.as_secs(),
        report.processed_files,
        report.failed_files,
        report.skipped_files,
        report.compression_ratio * 100.0,
        report.files_per_second,
        report.quality,
        report.mode
    );
    
    let report_path = "webpify_report.html";
    std::fs::write(report_path, html)?;
    println!("Report saved to: {report_path}");
    Ok(())
}
