use anyhow::{Context, Result};
use chrono::Utc;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::{
    ConversionReport, ReplaceInputMode,
    config::ConversionOptions,
    converter::ImageConverter,
    discovery::{DiscoveryOptions, FileInfo, discover_files},
    progress::ProgressReporter,
    stats::ConversionStats,
};

/// Core conversion engine that orchestrates the image conversion process
pub struct WebpifyCore {
    options: ConversionOptions,
    stats: ConversionStats,
    /// Output directories already created — skips a stat+mkdir per file.
    created_dirs: Mutex<HashSet<PathBuf>>,
}

impl WebpifyCore {
    /// Create a new core engine with the given options
    pub fn new(options: ConversionOptions) -> Self {
        Self {
            options,
            stats: ConversionStats::new(),
            created_dirs: Mutex::new(HashSet::new()),
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

        // Create output directory
        let output_dir = self.options.get_output_dir();
        std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

        // Scan input files with the shared discovery rules
        let files = discover_files(
            &self.options.input_dir,
            &DiscoveryOptions::from(&self.options),
        )?;

        if files.is_empty() {
            return Ok(self.base_report(start_time_utc, start_time, output_dir));
        }

        // Report progress
        if let Some(reporter) = &progress_reporter {
            reporter.set_total_files(files.len());
        }

        // Execute conversion. With an explicit thread count, run on a scoped
        // pool so repeated runs (GUI) actually honor the current setting —
        // the global pool can only be configured once per process.
        let result = match self.options.threads {
            Some(threads) => rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .context("Failed to build thread pool")?
                .install(|| self.convert_images(&files, &output_dir, progress_reporter)),
            None => self.convert_images(&files, &output_dir, progress_reporter),
        };
        result?;

        let mut report = self.base_report(start_time_utc, start_time, output_dir);
        report.total_files = files.len() as u64;
        report.processed_files = self.stats.processed_count.load(Ordering::Relaxed);
        report.failed_files = self.stats.error_count.load(Ordering::Relaxed);
        report.skipped_files = self.stats.skipped_count.load(Ordering::Relaxed);
        report.original_size = self.stats.original_size.load(Ordering::Relaxed);
        report.compressed_size = self.stats.compressed_size.load(Ordering::Relaxed);
        report.compression_ratio = self.stats.get_compression_ratio();
        report.files_per_second = self.stats.processed_count.load(Ordering::Relaxed) as f64
            / report.duration.as_secs_f64();
        report.bytes_per_second = (self.stats.compressed_size.load(Ordering::Relaxed) as f64
            / report.duration.as_secs_f64()) as u64;
        report.format_stats = self.stats.get_format_stats();
        report.errors = self.stats.get_errors();

        Ok(report)
    }

    /// Build the report skeleton shared by every run outcome.
    fn base_report(
        &self,
        start_time: chrono::DateTime<Utc>,
        start_instant: Instant,
        output_dir: PathBuf,
    ) -> ConversionReport {
        ConversionReport {
            start_time,
            end_time: Utc::now(),
            duration: start_instant.elapsed(),
            input_dir: self.options.input_dir.clone(),
            output_dir,
            thread_count: rayon::current_num_threads(),
            quality: self.options.quality,
            mode: format!("{:?}", self.options.mode),
            ..ConversionReport::default()
        }
    }

    /// Convert images with parallel processing
    fn convert_images(
        &self,
        files: &[FileInfo],
        output_dir: &Path,
        progress_reporter: Option<Box<dyn ProgressReporter>>,
    ) -> Result<()> {
        let converter = ImageConverter::new_with_dry_run(
            self.options.quality,
            &self.options.mode,
            &self.options.output_format,
            self.options.dry_run,
        );

        // Process files in parallel
        if let Some(reporter) = &progress_reporter {
            reporter.start_conversion();
        }

        files.par_iter().for_each(|file| {
            let input_path = &file.path;
            let result = self.process_single_file(&converter, input_path, output_dir);

            match result {
                Ok((original_size, compressed_size)) => {
                    self.stats.record_success(original_size, compressed_size);

                    // (0, 0) is the skip sentinel — not a conversion success
                    if original_size > 0
                        && let Some(reporter) = &progress_reporter
                    {
                        reporter.report_success(
                            &input_path.display().to_string(),
                            original_size,
                            compressed_size,
                        );
                    }

                    // Handle input file replacement
                    if !self.options.dry_run
                        && let Err(e) = self.handle_input_replacement(input_path)
                    {
                        log::warn!(
                            "Failed to handle input replacement for {}: {}",
                            input_path.display(),
                            e
                        );
                    }
                }
                Err(e) => {
                    let message = format!("{e:#}");
                    self.stats
                        .record_error(input_path.display().to_string(), message.clone());
                    if let Some(reporter) = &progress_reporter {
                        reporter.report_error(&input_path.display().to_string(), &message);
                    }
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

        if let Some(reporter) = &progress_reporter {
            reporter.finish_conversion();
        }

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

        // Create output directory if needed — once per directory, not per
        // file: the output root already exists, so this is one cached insert
        // instead of one stat+mkdir syscall per file.
        if let Some(parent) = output_path.parent()
            && self
                .created_dirs
                .lock()
                .unwrap()
                .insert(parent.to_path_buf())
        {
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
    fn calculate_output_path(&self, input_path: &Path, output_dir: &Path) -> Result<PathBuf> {
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
            output_dir.join(input_path.file_name().context("Failed to get filename")?)
        };

        // Change extension to the output format
        Ok(output_path.with_extension(self.options.output_format.extension()))
    }

    /// Handle input file replacement after successful conversion
    fn handle_input_replacement(&self, input_path: &Path) -> Result<()> {
        match self.options.replace_input {
            ReplaceInputMode::Off => Ok(()),
            ReplaceInputMode::Recycle => {
                trash::delete(input_path).with_context(|| {
                    format!("Failed to move to recycle bin: {}", input_path.display())
                })?;
                Ok(())
            }
            ReplaceInputMode::Delete => {
                std::fs::remove_file(input_path)
                    .with_context(|| format!("Failed to delete file: {}", input_path.display()))?;
                Ok(())
            }
        }
    }

    /// Get current conversion statistics
    pub fn get_stats(&self) -> &ConversionStats {
        &self.stats
    }
}
