/// Trait for reporting conversion progress
/// This allows different interfaces (CLI, GUI) to implement their own progress display
pub trait ProgressReporter: Send + Sync {
    /// Set the total number of files to be processed
    fn set_total_files(&self, total: usize);

    /// Update current progress
    fn update_progress(&self, processed: usize, failed: usize);

    /// Report that conversion has started
    fn start_conversion(&self) {}

    /// Report that conversion has finished
    fn finish_conversion(&self) {}

    /// Report an error for a specific file
    fn report_error(&self, _file_path: &str, _error: &str) {}

    /// Report successful conversion of a file
    fn report_success(&self, _file_path: &str, _original_size: u64, _compressed_size: u64) {}
}

/// A no-op progress reporter for when progress reporting is not needed
pub struct NoOpProgressReporter;

impl ProgressReporter for NoOpProgressReporter {
    fn set_total_files(&self, _total: usize) {}
    fn update_progress(&self, _processed: usize, _failed: usize) {}
}

/// Console-based progress reporter using indicatif
#[cfg(feature = "cli")]
pub struct ConsoleProgressReporter {
    progress_bar: indicatif::ProgressBar,
    multi_progress: indicatif::MultiProgress,
}

#[cfg(feature = "cli")]
impl Default for ConsoleProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleProgressReporter {
    pub fn new() -> Self {
        let multi_progress = indicatif::MultiProgress::new();
        let progress_bar = multi_progress.add(indicatif::ProgressBar::new(0));

        progress_bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        Self {
            progress_bar,
            multi_progress,
        }
    }

    pub fn get_multi_progress(&self) -> &indicatif::MultiProgress {
        &self.multi_progress
    }
}

#[cfg(feature = "cli")]
impl ProgressReporter for ConsoleProgressReporter {
    fn set_total_files(&self, total: usize) {
        self.progress_bar.set_length(total as u64);
    }

    fn update_progress(&self, processed: usize, _failed: usize) {
        self.progress_bar.set_position(processed as u64);
    }

    fn start_conversion(&self) {
        self.progress_bar.set_message("Converting images...");
    }

    fn finish_conversion(&self) {
        self.progress_bar
            .finish_with_message("Conversion completed!");
    }

    fn report_error(&self, file_path: &str, error: &str) {
        self.progress_bar
            .println(format!("❌ Error processing {file_path}: {error}"));
    }

    fn report_success(&self, file_path: &str, original_size: u64, compressed_size: u64) {
        let ratio = if original_size > 0 {
            ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        self.progress_bar.println(format!(
            "✅ {} -> {} ({:.1}% reduction)",
            file_path,
            humansize::format_size(compressed_size, humansize::DECIMAL),
            ratio
        ));
    }
}
