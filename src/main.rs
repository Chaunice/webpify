use anyhow::Result;
use clap::{CommandFactory, Parser, ValueEnum};
use std::path::PathBuf;

// Use the library
use webpify::{
    config::ConversionOptions, 
    generate_report, 
    CompressionMode, 
    ConversionReport,
    ReplaceInputMode, 
    ReportFormat, 
    WebpifyCore,
};

#[cfg(feature = "cli")]
use webpify::progress::ConsoleProgressReporter;

/// webpify - High-performance batch image to WebP converter
///
/// Efficiently converts various image formats to WebP with compression optimization and parallel processing
#[derive(Parser)]
#[command(name = "webpify")]
#[command(about = "webpify - High-performance batch WebP converter")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Haobo Zhang")]
#[command(long_about = r#"
webpify is a high-performance tool designed for large-scale image conversion to WebP format.

Features:
• Multi-threaded parallel processing leveraging full CPU power
• Real-time progress display with detailed statistics
• Smart format detection and batch conversion
• WebP compression optimization for significant space savings
• Recursive directory scanning with nested folder support
• Comprehensive conversion reports and performance analysis
"#)]
#[command(before_help = r#"
                    __                        ___             
                   /\ \                __   /'___\            
 __  __  __     __ \ \ \____   _____  /\_\ /\ \__/  __  __    
/\ \/\ \/\ \  /'__`\\ \ '__`\ /\ '__`\\/\ \\ \ ,__\/\ \/\ \   
\ \ \_/ \_/ \/\  __/ \ \ \L\ \\ \ \L\ \\ \ \\ \ \_/\ \ \_\ \  
 \ \___x___/'\ \____\ \ \_,__/ \ \ ,__/ \ \_\\ \_\  \/`____ \ 
  \/__//__/   \/____/  \/___/   \ \ \/   \/_/ \/_/   `/___/> \
                                 \ \_\                  /\___/
                                  \/_/                  \/__/ 
"#)]
pub struct Args {
    /// Input directory path
    #[arg(short, long, value_name = "DIR")]
    pub input: PathBuf,

    /// Output directory path (defaults to input_dir/webp_output)
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,

    /// WebP compression quality (0-100)
    #[arg(short, long, default_value = "80", value_name = "QUALITY")]
    pub quality: u8,

    /// Number of parallel threads (defaults to CPU core count for I/O optimization)
    #[arg(short, long, value_name = "NUM")]
    pub threads: Option<usize>,

    /// Compression mode
    #[arg(short, long, default_value = "lossless", value_enum)]
    pub mode: CompressionModeArg,

    /// Supported input formats (defaults to common formats)
    #[arg(long, value_delimiter = ',', default_values = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"])]
    pub formats: Vec<String>,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,

    /// Preserve original directory structure
    #[arg(long, default_value = "true")]
    pub preserve_structure: bool,

    /// Maximum file size limit (MB)
    #[arg(long, value_name = "SIZE")]
    pub max_size: Option<u64>,

    /// Minimum file size limit (KB)
    #[arg(long, default_value = "1", value_name = "SIZE")]
    pub min_size: u64,

    /// Enable pre-processing scan
    #[arg(long, default_value = "true")]
    pub prescan: bool,

    /// Verbose output mode
    #[arg(short, long)]
    pub verbose: bool,

    /// Quiet mode (results only)
    #[arg(long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Generate conversion report
    #[arg(long)]
    pub report: bool,

    /// Report output format
    #[arg(long, default_value = "json", value_enum)]
    pub report_format: ReportFormatArg,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Use a predefined configuration profile
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<String>,

    /// How to handle input files after successful conversion [off: keep, recycle: move to recycle bin, delete: permanently delete]
    #[arg(long, value_enum, default_value = "off")]
    pub replace_input: ReplaceInputModeArg,

    /// Force re-encoding of WebP files (by default, .webp files are skipped)
    #[arg(long, default_value_t = false)]
    pub reencode_webp: bool,

    /// Dry run mode - preview operations without making changes
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CompressionModeArg {
    /// Lossless compression (larger files but perfect quality)
    Lossless,
    /// Lossy compression (smaller files with slight quality loss)
    Lossy,
    /// Auto mode (intelligently choose based on image characteristics)
    Auto,
}

impl From<CompressionModeArg> for CompressionMode {
    fn from(mode: CompressionModeArg) -> Self {
        match mode {
            CompressionModeArg::Lossless => CompressionMode::Lossless,
            CompressionModeArg::Lossy => CompressionMode::Lossy,
            CompressionModeArg::Auto => CompressionMode::Auto,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReportFormatArg {
    Json,
    Csv,
    Html,
}

impl From<ReportFormatArg> for ReportFormat {
    fn from(format: ReportFormatArg) -> Self {
        match format {
            ReportFormatArg::Json => ReportFormat::Json,
            ReportFormatArg::Csv => ReportFormat::Csv,
            ReportFormatArg::Html => ReportFormat::Html,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReplaceInputModeArg {
    /// Do not delete input files (default)
    Off,
    /// Move input files to recycle bin after successful conversion
    Recycle,
    /// Permanently delete input files after successful conversion
    Delete,
}

impl From<ReplaceInputModeArg> for ReplaceInputMode {
    fn from(mode: ReplaceInputModeArg) -> Self {
        match mode {
            ReplaceInputModeArg::Off => ReplaceInputMode::Off,
            ReplaceInputModeArg::Recycle => ReplaceInputMode::Recycle,
            ReplaceInputModeArg::Delete => ReplaceInputMode::Delete,
        }
    }
}

fn main() -> Result<()> {
    if std::env::args().len() == 1 {
        print_ascii_banner();
        Args::command().print_help()?;
        println!();
        std::process::exit(0);
    }

    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else if !args.quiet {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Convert CLI args to library configuration
    let mut options = ConversionOptions::new(args.input)
        .with_quality(args.quality)
        .with_mode(args.mode.into())
        .with_dry_run(args.dry_run)
        .with_overwrite(args.overwrite);

    if let Some(output) = args.output {
        options = options.with_output_dir(output);
    }

    if let Some(threads) = args.threads {
        options = options.with_threads(threads);
    }

    // Create and run the core engine
    let mut core = WebpifyCore::new(options);

    #[cfg(feature = "cli")]
    let progress_reporter = if !args.quiet {
        let reporter = ConsoleProgressReporter::new();
        if !args.quiet {
            print_ascii_banner();
        }
        Some(Box::new(reporter) as Box<dyn webpify::ProgressReporter>)
    } else {
        None
    };

    #[cfg(not(feature = "cli"))]
    let progress_reporter = None;

    // Run conversion
    let report = core.run_with_progress(progress_reporter)?;

    // Generate report if requested
    if args.report {
        generate_report(&report, &args.report_format.into())?;
    }

    // Print summary if not quiet
    if !args.quiet {
        print_results_summary(&report);
    }

    Ok(())
}

fn print_ascii_banner() {
    println!(r#"
                    __                        ___             
                   /\ \                __   /'___\            
 __  __  __     __ \ \ \____   _____  /\_\ /\ \__/  __  __    
/\ \/\ \/\ \  /'__`\\ \ '__`\ /\ '__`\\/\ \\ \ ,__\/\ \/\ \   
\ \ \_/ \_/ \/\  __/ \ \ \L\ \\ \ \L\ \\ \ \\ \ \_/\ \ \_\ \  
 \ \___x___/'\ \____\ \ \_,__/ \ \ ,__/ \ \_\\ \_\  \/`____ \ 
  \/__//__/   \/____/  \/___/   \ \ \/   \/_/ \/_/   `/___/> \
                                 \ \_\                  /\___/
                                  \/_/                  \/__/ 

        High-Performance Batch WebP Converter v{}
    "#, env!("CARGO_PKG_VERSION"));
}

fn print_results_summary(report: &ConversionReport) {
    use humansize::{format_size, DECIMAL};
    
    println!("\n🎉 Conversion completed!");
    println!("📊 Results Summary:");
    println!("  ✅ Processed: {} files", report.processed_files);
    if report.failed_files > 0 {
        println!("  ❌ Failed: {} files", report.failed_files);
    }
    if report.skipped_files > 0 {
        println!("  ⏭️ Skipped: {} files", report.skipped_files);
    }
    
    if report.original_size > 0 {
        println!("\n💾 Space Analysis:");
        println!("  📦 Original size: {}", format_size(report.original_size, DECIMAL));
        println!("  🗜️ Compressed size: {}", format_size(report.compressed_size, DECIMAL));
        println!("  💾 Space saved: {:.1}%", report.compression_ratio * 100.0);
    }
    
    println!("\n⏱️ Performance:");
    println!("  🕐 Duration: {:.1}s", report.duration.as_secs_f64());
    println!("  🚀 Speed: {:.1} files/sec", report.files_per_second);
    println!("  🧵 Threads used: {}", report.thread_count);
    
    if !report.errors.is_empty() && report.errors.len() <= 5 {
        println!("\n❌ Errors:");
        for error in &report.errors {
            println!("  • {}", error);
        }
    } else if report.errors.len() > 5 {
        println!("\n❌ {} errors occurred (use --report for full details)", report.errors.len());
    }
}
