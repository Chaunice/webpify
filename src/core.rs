use anyhow::{Context, Result};
use chrono::Utc;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;
use walkdir::WalkDir;

use crate::{
    config::ConversionOptions, converter::ImageConverter, progress::ProgressReporter,
    stats::ConversionStats, utils::is_valid_image_file, ConversionReport, ReplaceInputMode,
};

/// Core conversion engine that orchestrates the image conversion process
pub struct WebpifyCore {
    options: ConversionOptions,
    stats: ConversionStats,
}

impl WebpifyCore {
    /// Create a new core engine with the given options
    pub fn new(options: ConversionOptions) -> Self {
        Self {
            options,
            stats: ConversionStats::new(),
        }
    }

    /// Run the complete conversion process
    pub fn run(&mut self) -> Result<ConversionReport> {
        self.run_with_progress(None)
    }

    /// Run the conversion process with progress reporting
    pub fn run_with_progress(
        &mut self,
        progress_reporter: Option<Box<dyn ProgressReporter>>,
    ) -> Result<ConversionReport> {
        let start_time = Instant::now();
        let start_time_utc = Utc::now();

        // Setup thread pool (only if not already initialized)
        if let Some(threads) = self.options.threads {
            // Check if global pool is already initialized by trying to build a new one
            if rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build_global()
                .is_err()
            {
                // Thread pool already exists, just log a warning
                log::debug!("Thread pool already initialized, using existing configuration");
            }
        }

        // Create output directory
        let output_dir = self.options.get_output_dir();
        std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

        // Start timing
        self.stats.start_timer();

        // Scan input files
        let files = if self.options.prescan {
            self.scan_input_files()?
        } else {
            self.scan_files_streaming()?
        };

        if files.is_empty() {
            return Ok(self.create_empty_report(start_time_utc, start_time, output_dir));
        }

        // Report progress
        if let Some(reporter) = &progress_reporter {
            reporter.set_total_files(files.len());
        }

        // Execute conversion
        self.convert_images(&files, &output_dir, progress_reporter)?;

        let duration = start_time.elapsed();
        let end_time_utc = Utc::now();

        // Create final report
        Ok(ConversionReport {
            start_time: start_time_utc,
            end_time: end_time_utc,
            duration,
            input_dir: self.options.input_dir.clone(),
            output_dir,
            total_files: files.len() as u64,
            processed_files: self.stats.processed_count.load(Ordering::Relaxed),
            failed_files: self.stats.error_count.load(Ordering::Relaxed),
            skipped_files: self.stats.skipped_count.load(Ordering::Relaxed),
            original_size: self.stats.original_size.load(Ordering::Relaxed),
            compressed_size: self.stats.compressed_size.load(Ordering::Relaxed),
            compression_ratio: self.stats.get_compression_ratio(),
            files_per_second: self.stats.processed_count.load(Ordering::Relaxed) as f64
                / duration.as_secs_f64(),
            bytes_per_second: (self.stats.compressed_size.load(Ordering::Relaxed) as f64
                / duration.as_secs_f64()) as u64,
            thread_count: rayon::current_num_threads(),
            quality: self.options.quality,
            mode: format!("{:?}", self.options.mode),
            format_stats: self.stats.get_format_stats(),
            errors: self.stats.get_errors(),
        })
    }

    /// Scan input files with progress updates
    fn scan_input_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.options.input_dir)
            .follow_links(false)
            .into_iter()
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if !is_valid_image_file(path) {
                continue;
            }

            // Check file extension
            if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                let ext_lower = extension.to_lowercase();
                if !self.options.formats.contains(&ext_lower) {
                    continue;
                }

                // Skip WebP files unless re-encoding is enabled
                if ext_lower == "webp" && !self.options.reencode_webp {
                    continue;
                }
            }

            // Check file size constraints
            if let Ok(metadata) = std::fs::metadata(path) {
                let file_size = metadata.len();

                if file_size < self.options.min_size * 1024 {
                    continue;
                }

                if let Some(max_size) = self.options.max_size {
                    if file_size > max_size * 1024 * 1024 {
                        continue;
                    }
                }
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    /// Streaming file scan (alternative implementation)
    fn scan_files_streaming(&self) -> Result<Vec<PathBuf>> {
        // For now, use the same implementation as scan_input_files
        // This could be optimized for very large directories
        self.scan_input_files()
    }

    /// Convert images with parallel processing
    fn convert_images(
        &self,
        files: &[PathBuf],
        output_dir: &Path,
        progress_reporter: Option<Box<dyn ProgressReporter>>,
    ) -> Result<()> {
        let converter = ImageConverter::new_with_dry_run(
            self.options.quality,
            &self.options.mode,
            self.options.dry_run,
        );

        // Process files in parallel
        files.par_iter().for_each(|input_path| {
            let result = self.process_single_file(&converter, input_path, output_dir);

            match result {
                Ok((original_size, compressed_size)) => {
                    self.stats.record_success(original_size, compressed_size);

                    // Handle input file replacement
                    if !self.options.dry_run {
                        if let Err(e) = self.handle_input_replacement(input_path) {
                            log::warn!("Failed to handle input replacement for {}: {}", 
                                     input_path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    self.stats.record_error(
                        input_path.display().to_string(),
                        format!("{e:#}"),
                    );
                    log::error!("Failed to convert {}: {:#}", input_path.display(), e);
                }
            }

            // Report progress
            if let Some(reporter) = &progress_reporter {
                reporter.update_progress(
                    self.stats.processed_count.load(Ordering::Relaxed) as usize,
                    self.stats.error_count.load(Ordering::Relaxed) as usize,
                );
            }
        });

        Ok(())
    }

    /// Process a single file conversion
    fn process_single_file(
        &self,
        converter: &ImageConverter,
        input_path: &Path,
        output_dir: &Path,
    ) -> Result<(u64, u64)> {
        let output_path = self.calculate_output_path(input_path, output_dir)?;

        // Check if output file already exists
        if output_path.exists() && !self.options.overwrite {
            self.stats.record_skip();
            return Ok((0, 0)); // Skip without error
        }

        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Record format statistics
        if let Some(extension) = input_path.extension().and_then(|ext| ext.to_str()) {
            self.stats.record_format(&extension.to_lowercase());
        }

        // Perform conversion
        converter.convert_to_webp(input_path, &output_path)
    }

    /// Calculate the output path for a given input file
    fn calculate_output_path(
        &self,
        input_path: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let relative_path = input_path
            .strip_prefix(&self.options.input_dir)
            .with_context(|| {
                format!(
                    "Input path {} is not under input directory {}",
                    input_path.display(),
                    self.options.input_dir.display()
                )
            })?;

        let output_path = if self.options.preserve_structure {
            output_dir.join(relative_path)
        } else {
            output_dir.join(
                input_path
                    .file_name()
                    .context("Failed to get filename")?,
            )
        };

        // Change extension to .webp
        Ok(output_path.with_extension("webp"))
    }

    /// Handle input file replacement after successful conversion
    fn handle_input_replacement(&self, input_path: &Path) -> Result<()> {
        match self.options.replace_input {
            ReplaceInputMode::Off => Ok(()),
            ReplaceInputMode::Recycle => {
                trash::delete(input_path)
                    .with_context(|| format!("Failed to move to recycle bin: {}", input_path.display()))?;
                Ok(())
            }
            ReplaceInputMode::Delete => {
                std::fs::remove_file(input_path)
                    .with_context(|| format!("Failed to delete file: {}", input_path.display()))?;
                Ok(())
            }
        }
    }

    /// Create an empty report for when no files are found
    fn create_empty_report(
        &self,
        start_time_utc: chrono::DateTime<Utc>,
        start_time: Instant,
        output_dir: PathBuf,
    ) -> ConversionReport {
        let duration = start_time.elapsed();
        let end_time_utc = Utc::now();

        ConversionReport {
            start_time: start_time_utc,
            end_time: end_time_utc,
            duration,
            input_dir: self.options.input_dir.clone(),
            output_dir,
            total_files: 0,
            processed_files: 0,
            failed_files: 0,
            skipped_files: 0,
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
            files_per_second: 0.0,
            bytes_per_second: 0,
            thread_count: rayon::current_num_threads(),
            quality: self.options.quality,
            mode: format!("{:?}", self.options.mode),
            format_stats: std::collections::HashMap::new(),
            errors: vec!["No supported image files found in the specified directory".to_string()],
        }
    }

    /// Get current conversion statistics
    pub fn get_stats(&self) -> &ConversionStats {
        &self.stats
    }
}
