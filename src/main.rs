use anyhow::{Context, Result};
use clap::{Parser, ValueEnum, CommandFactory};
use chrono::{DateTime, Utc};
use humansize::{format_size, DECIMAL};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn, error};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

mod converter;
mod stats;
mod utils;

use converter::ImageConverter;
use stats::ConversionStats;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: Option<GeneralConfig>,
    pub compression: Option<CompressionConfig>,
    pub performance: Option<PerformanceConfig>,
    pub filtering: Option<FilteringConfig>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub input_dir: Option<String>,
    pub output_dir: Option<String>,
    pub preserve_structure: Option<bool>,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CompressionConfig {
    pub quality: Option<u8>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PerformanceConfig {
    pub threads: Option<usize>,
    pub prescan: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FilteringConfig {
    pub formats: Option<Vec<String>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub generate_report: Option<bool>,
    pub report_format: Option<String>,
}

/// webpify - High-performance batch image to WebP converter
/// 
/// Efficiently converts various image formats to WebP with compression optimization and parallel processing
#[derive(Parser)]
#[command(name = "webpify")]
#[command(about = "webpify - High-performance batch WebP converter")]
#[command(version = "0.1.0")]
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
#[command(before_help = 
r#"
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
    pub mode: CompressionMode,

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

    /// Performance mode: fast, balanced, quality
    #[arg(long, default_value = "balanced")]
    pub performance: String,

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
    pub report_format: ReportFormat,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Ultra-fast mode (trades quality for speed)
    #[arg(long)]
    pub ultra_fast: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CompressionMode {
    /// Lossless compression (larger files but perfect quality)
    Lossless,
    /// Lossy compression (smaller files with slight quality loss)
    Lossy,
    /// Auto mode (intelligently choose based on image characteristics)
    Auto,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReportFormat {
    Json,
    Csv,
    Html,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[tokio::main]
async fn main() -> Result<()> {

    if std::env::args().len() == 1 {
        // print_ascii_banner();
        Args::command().print_help()?;
        println!();
        std::process::exit(0);
    }

    let mut args = Args::parse();
    
    // Load config file if specified
    if let Some(config_path) = args.config.clone() {
        load_config(&mut args, &config_path)?;
    }
    
    // Initialize logging system
    init_logging(&args)?;
    
    // Validate arguments
    validate_args(&args)?;
    
    // 设置线程池
    setup_thread_pool(args.threads);
    
    // Create output directory
    let output_dir = get_output_dir(&args)?;
    std::fs::create_dir_all(&output_dir)
        .context("Failed to create output directory")?;
    
    if !args.quiet {
        print_ascii_banner();
        print_ascii_config(&args, &output_dir);
    }
    
    let start_time = Instant::now();
    let start_time_utc = Utc::now();
    
    // Scan input files
    let files = if args.prescan {
        scan_input_files(&args).await?
    } else {
        scan_files_streaming(&args)?
    };
    
    if files.is_empty() {
        warn!("No supported image files found in the specified directory");
        
        // Generate empty report if requested
        if args.report {
            let end_time_utc = Utc::now();
            let duration = start_time.elapsed();
            
            let empty_report = ConversionReport {
                start_time: start_time_utc,
                end_time: end_time_utc,
                duration,
                input_dir: args.input.clone(),
                output_dir: output_dir.clone(),
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
                quality: args.quality,
                mode: format!("{:?}", args.mode),
                format_stats: HashMap::new(),
                errors: vec!["No supported image files found in the specified directory".to_string()],
            };
            
            generate_report(&empty_report, &args.report_format)?;
        }
        
        return Ok(());
    }
    
    if !args.quiet {
        info!("Found {} files, starting conversion...", files.len());
    }
    
    // Execute conversion
    let stats = convert_images(&args, &files, &output_dir).await?;
    
    let duration = start_time.elapsed();
    let end_time_utc = Utc::now();
    
    // Display results
    if !args.quiet {
        print_ascii_results(&stats, duration);
    }
    
    // Generate report
    if args.report {
        let report = ConversionReport {
            start_time: start_time_utc,
            end_time: end_time_utc,
            duration,
            input_dir: args.input.clone(),
            output_dir: output_dir.clone(),
            total_files: files.len() as u64,
            processed_files: stats.processed_count.load(Ordering::Relaxed),
            failed_files: stats.error_count.load(Ordering::Relaxed),
            skipped_files: stats.skipped_count.load(Ordering::Relaxed),
            original_size: stats.original_size.load(Ordering::Relaxed),
            compressed_size: stats.compressed_size.load(Ordering::Relaxed),
            compression_ratio: stats.get_compression_ratio(),
            files_per_second: stats.processed_count.load(Ordering::Relaxed) as f64 / duration.as_secs_f64(),
            bytes_per_second: (stats.compressed_size.load(Ordering::Relaxed) as f64 / duration.as_secs_f64()) as u64,
            thread_count: rayon::current_num_threads(),
            quality: args.quality,
            mode: format!("{:?}", args.mode),
            format_stats: stats.get_format_stats(),
            errors: stats.get_errors(),
        };
        
        generate_report(&report, &args.report_format)?;
    }
    
    Ok(())
}

fn load_config(args: &mut Args, config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        warn!("Config file not found: {}", config_path.display());
        return Ok(());
    }
    
    let config_content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
    
    // Apply config values if they weren't explicitly set via CLI
    if let Some(general) = &config.general {
        if args.input.as_os_str().is_empty() {
            if let Some(input_dir) = &general.input_dir {
                args.input = PathBuf::from(input_dir);
            }
        }
        
        if args.output.is_none() {
            if let Some(output_dir) = &general.output_dir {
                args.output = Some(PathBuf::from(output_dir));
            }
        }
        
        if let Some(preserve) = general.preserve_structure {
            if !args.preserve_structure {
                args.preserve_structure = preserve;
            }
        }
        
        if let Some(overwrite) = general.overwrite {
            if !args.overwrite {
                args.overwrite = overwrite;
            }
        }
    }
    
    if let Some(compression) = &config.compression {
        if let Some(quality) = compression.quality {
            if args.quality == 80 { // default value
                args.quality = quality;
            }
        }
        
        if let Some(mode_str) = &compression.mode {
            match mode_str.to_lowercase().as_str() {
                "lossless" => args.mode = CompressionMode::Lossless,
                "lossy" => args.mode = CompressionMode::Lossy,
                "auto" => args.mode = CompressionMode::Auto,
                _ => warn!("Invalid compression mode in config: {}", mode_str),
            }
        }
    }
    
    if let Some(performance) = &config.performance {
        if args.threads.is_none() {
            if let Some(threads) = performance.threads {
                if threads > 0 {
                    args.threads = Some(threads);
                }
            }
        }
        
        if let Some(prescan) = performance.prescan {
            args.prescan = prescan;
        }
    }
    
    if let Some(filtering) = &config.filtering {
        if let Some(formats) = &filtering.formats {
            args.formats = formats.clone();
        }
        
        if let Some(min_size) = filtering.min_size {
            if args.min_size == 1 { // default value
                args.min_size = min_size;
            }
        }
        
        if let Some(max_size) = filtering.max_size {
            if max_size > 0 {
                args.max_size = Some(max_size);
            }
        }
    }
    
    if let Some(output) = &config.output {
        if let Some(verbose) = output.verbose {
            if !args.verbose {
                args.verbose = verbose;
            }
        }
        
        if let Some(quiet) = output.quiet {
            if !args.quiet {
                args.quiet = quiet;
            }
        }
        
        if let Some(generate_report) = output.generate_report {
            if !args.report {
                args.report = generate_report;
            }
        }
        
        if let Some(report_format_str) = &output.report_format {
            match report_format_str.to_lowercase().as_str() {
                "json" => args.report_format = ReportFormat::Json,
                "csv" => args.report_format = ReportFormat::Csv,
                "html" => args.report_format = ReportFormat::Html,
                _ => warn!("Invalid report format in config: {}", report_format_str),
            }
        }
    }
    
    info!("Loaded configuration from: {}", config_path.display());
    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    let level = if args.verbose {
        "debug"
    } else if args.quiet {
        "error"
    } else {
        "info"
    };
    
    std::env::set_var("RUST_LOG", level);
    env_logger::init();
    Ok(())
}

fn validate_args(args: &Args) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("Input directory does not exist: {}", args.input.display());
    }
    
    if !args.input.is_dir() {
        anyhow::bail!("Input path is not a directory: {}", args.input.display());
    }
    
    if args.quality > 100 {
        anyhow::bail!("Quality must be between 0-100");
    }
    
    if let Some(threads) = args.threads {
        if threads == 0 {
            anyhow::bail!("Thread count must be greater than 0");
        }
    }
    
    Ok(())
}

fn setup_thread_pool(threads: Option<usize>) {
    let optimal_threads = match threads {
        Some(num_threads) => num_threads,
        None => {
            // For I/O-heavy workloads, use more threads than CPU cores
            let cpu_cores = num_cpus::get();
            let io_threads = cpu_cores; // Double for I/O optimization
            std::cmp::min(io_threads, 32) // Cap at 32 to avoid thread overhead
        }
    };
    
    rayon::ThreadPoolBuilder::new()
        .num_threads(optimal_threads)
        .build_global()
        .expect("Failed to setup thread pool");
    
    info!("Using {} threads for optimal I/O performance", optimal_threads);
}

fn get_output_dir(args: &Args) -> Result<PathBuf> {
    match &args.output {
        Some(output) => Ok(output.clone()),
        None => Ok(args.input.join("webp_output")),
    }
}

async fn scan_input_files(args: &Args) -> Result<Vec<PathBuf>> {
    let supported_extensions: Vec<String> = args.formats
        .iter()
        .map(|f| f.to_lowercase())
        .collect();
    
    if !args.quiet {
        info!("Scanning directory: {}", args.input.display());
    }
    
    let files: Vec<PathBuf> = WalkDir::new(&args.input)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            
            if !path.is_file() {
                return None;
            }
            
            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase())?;
            
            if !supported_extensions.contains(&extension) {
                return None;
            }
            
            // Check file size
            if let Ok(metadata) = std::fs::metadata(path) {
                let size_kb = metadata.len() / 1024;
                if size_kb < args.min_size {
                    return None;
                }
                
                if let Some(max_size) = args.max_size {
                    let size_mb = metadata.len() / 1024 / 1024;
                    if size_mb > max_size {
                        return None;
                    }
                }
            }
            
            Some(path.to_path_buf())
        })
        .collect();
    
    Ok(files)
}

fn scan_files_streaming(args: &Args) -> Result<Vec<PathBuf>> {
    // Streaming scan implementation for very large directories
    // For this simplified version, we use the regular scan
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(scan_input_files(args))
}

async fn convert_images(
    args: &Args,
    files: &[PathBuf],
    output_dir: &Path,
) -> Result<ConversionStats> {
    let stats = ConversionStats::new();
    let converter = if args.ultra_fast {
        ImageConverter::new_with_speed(args.quality, &args.mode, true)
    } else {
        ImageConverter::new(args.quality, &args.mode)
    };
    
    // Performance optimization: Pre-create all output directories
    if !args.quiet {
        info!("Pre-creating output directories for optimal performance...");
    }
    let created_dirs = pre_create_directories(files, output_dir, &args.input, args.preserve_structure)?;
    
    // Create optimized progress bar with reduced update frequency
    let multi_progress = MultiProgress::new();
    let main_progress = multi_progress.add(ProgressBar::new(files.len() as u64));
    main_progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Enable real-time drawing for more responsive UI
    main_progress.enable_steady_tick(Duration::from_millis(100));
    
    let stats_clone = stats.clone();
    let progress_clone = main_progress.clone();
    let created_dirs = Arc::new(created_dirs);
    
    // High-performance parallel processing with optimized chunk size
    let chunk_size = if args.ultra_fast {
        // Ultra-fast mode: larger chunks, less frequent updates for maximum speed
        std::cmp::max(10, files.len() / (rayon::current_num_threads()))
    } else {
        // Normal mode: smaller chunks for more responsive UI
        std::cmp::max(1, files.len() / (rayon::current_num_threads()))
    };
    
    files
        .par_chunks(chunk_size)
        .for_each(|chunk| {
            let mut local_processed = 0;
            let update_frequency = if args.ultra_fast { 
                std::cmp::max(5, chunk.len() / 4) // Update every 5 files or 25% of chunk in ultra-fast
            } else { 
                1 // Update every file in normal mode
            };
            
            for (i, input_path) in chunk.iter().enumerate() {
                let result = process_single_image_optimized(
                    input_path,
                    output_dir,
                    &args.input,
                    &converter,
                    args.preserve_structure,
                    args.overwrite,
                    &created_dirs,
                );
                
                match result {
                    Ok((original_size, compressed_size)) => {
                        stats_clone.record_success(original_size, compressed_size);
                        local_processed += 1;
                    },
                    Err(e) => {
                        stats_clone.record_error(format!("{}: {}", input_path.display(), e));
                        if args.verbose {
                            error!("Failed to process {}: {}", input_path.display(), e);
                        }
                    }
                }
                
                // More frequent progress updates for better user experience
                if !args.quiet && (i + 1) % update_frequency == 0 {
                    progress_clone.inc(local_processed);
                    let current_pos = progress_clone.position();
                    let total_files = progress_clone.length().unwrap_or(0);
                    let percentage = if total_files > 0 { 
                        (current_pos as f64 / total_files as f64 * 100.0) as u32 
                    } else { 0 };
                    progress_clone.set_message(format!("{}% - Processing batch of {} files", percentage, chunk.len()));
                    local_processed = 0; // Reset counter after update
                }
            }
            
            // Final update for any remaining files in the chunk
            if !args.quiet && local_processed > 0 {
                progress_clone.inc(local_processed);
                let current_pos = progress_clone.position();
                let total_files = progress_clone.length().unwrap_or(0);
                let percentage = if total_files > 0 { 
                    (current_pos as f64 / total_files as f64 * 100.0) as u32 
                } else { 0 };
                progress_clone.set_message(format!("{}% - Completed batch of {} files", percentage, chunk.len()));
            }
        });
    
    if !args.quiet {
        main_progress.finish_with_message("Conversion completed!");
    }
    
    Ok(stats)
}

#[allow(dead_code)]
fn process_single_image(
    input_path: &Path,
    output_dir: &Path,
    input_root: &Path,
    converter: &ImageConverter,
    preserve_structure: bool,
    overwrite: bool,
) -> Result<(u64, u64)> {
    let input_metadata = std::fs::metadata(input_path)?;
    let original_size = input_metadata.len();
    
    // Calculate output path
    let output_path = if preserve_structure {
        let relative_path = input_path.strip_prefix(input_root)?;
        let mut output_path = output_dir.join(relative_path);
        output_path.set_extension("webp");
        output_path
    } else {
        let filename = input_path.file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
        output_dir.join(format!("{}.webp", filename.to_string_lossy()))
    };
    
    // Check if overwrite is needed
    if output_path.exists() && !overwrite {
        return Err(anyhow::anyhow!("File exists and overwrite mode is disabled"));
    }
    
    // Create output directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Convert image
    converter.convert_to_webp(input_path, &output_path)?;
    
    let compressed_size = std::fs::metadata(&output_path)?.len();
    
    Ok((original_size, compressed_size))
}

fn pre_create_directories(
    files: &[PathBuf],
    output_dir: &Path,
    input_root: &Path,
    preserve_structure: bool,
) -> Result<HashSet<PathBuf>> {
    let mut dirs_to_create = HashSet::new();
    
    // Collect all unique directory paths
    for input_path in files {
        let output_path = if preserve_structure {
            let relative_path = input_path.strip_prefix(input_root)?;
            let mut output_path = output_dir.join(relative_path);
            output_path.set_extension("webp");
            output_path
        } else {
            let filename = input_path.file_stem()
                .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
            output_dir.join(format!("{}.webp", filename.to_string_lossy()))
        };
        
        if let Some(parent) = output_path.parent() {
            dirs_to_create.insert(parent.to_path_buf());
        }
    }
    
    // Batch create all directories
    dirs_to_create.par_iter().for_each(|dir| {
        if let Err(e) = std::fs::create_dir_all(dir) {
            error!("Failed to create directory {}: {}", dir.display(), e);
        }
    });
    
    Ok(dirs_to_create)
}

fn process_single_image_optimized(
    input_path: &Path,
    output_dir: &Path,
    input_root: &Path,
    converter: &ImageConverter,
    preserve_structure: bool,
    overwrite: bool,
    _created_dirs: &Arc<HashSet<PathBuf>>, // Pre-created directories
) -> Result<(u64, u64)> {
    let input_metadata = std::fs::metadata(input_path)?;
    let original_size = input_metadata.len();
    
    // Calculate output path (same logic but directory already exists)
    let output_path = if preserve_structure {
        let relative_path = input_path.strip_prefix(input_root)?;
        let mut output_path = output_dir.join(relative_path);
        output_path.set_extension("webp");
        output_path
    } else {
        let filename = input_path.file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
        output_dir.join(format!("{}.webp", filename.to_string_lossy()))
    };
    
    // Check if overwrite is needed
    if output_path.exists() && !overwrite {
        return Err(anyhow::anyhow!("File exists and overwrite mode is disabled"));
    }
    
    // Convert image (directory already pre-created)
    converter.convert_to_webp(input_path, &output_path)?;
    
    let compressed_size = std::fs::metadata(&output_path)?.len();
    
    Ok((original_size, compressed_size))
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
"#);
}

fn print_ascii_config(args: &Args, output_dir: &Path) {
    println!(">> Conversion Configuration:");
    println!("   Input:       {}", args.input.display());
    println!("   Output:      {}", output_dir.display());
    println!("   Quality:     {}", args.quality);
    println!("   Mode:        {:?}", args.mode);
    println!("   Threads:     {}", rayon::current_num_threads());
    println!("   Formats:     {}", args.formats.join(", "));
    println!();
}

fn print_results(stats: &ConversionStats, duration: Duration) {
    let processed = stats.processed_count.load(Ordering::Relaxed);
    let errors = stats.error_count.load(Ordering::Relaxed);
    let original_size = stats.original_size.load(Ordering::Relaxed);
    let compressed_size = stats.compressed_size.load(Ordering::Relaxed);
    
    println!("\n>> Conversion Results:");
    println!("   Processed:   {} files", processed);
    println!("   Failed:      {} files", errors);
    println!("   Original:    {}", format_size(original_size, DECIMAL));
    println!("   Compressed:  {}", format_size(compressed_size, DECIMAL));
    println!("   Ratio:       {:.1}%", stats.get_compression_ratio() * 100.0);
    println!("   Saved:       {}", format_size(original_size.saturating_sub(compressed_size), DECIMAL));
    println!("   Speed:       {:.1} files/sec", processed as f64 / duration.as_secs_f64());
    println!("   Duration:    {:.2?}", duration);
}

fn print_ascii_results(stats: &ConversionStats, duration: Duration) {
    print_results(stats, duration); // Just call the updated function
}

fn generate_report(report: &ConversionReport, format: &ReportFormat) -> Result<()> {
    match format {
        ReportFormat::Json => {
            let json = serde_json::to_string_pretty(report)?;
            std::fs::write("conversion_report.json", json)?;
            info!("Generated JSON report: conversion_report.json");
        },
        ReportFormat::Csv => {
            generate_csv_report(report)?;
            info!("Generated CSV report: conversion_report.csv");
        },
        ReportFormat::Html => {
            generate_html_report(report)?;
            info!("Generated HTML report: conversion_report.html");
        },
    }
    Ok(())
}

fn generate_csv_report(report: &ConversionReport) -> Result<()> {
    let mut wtr = csv::Writer::from_path("conversion_report.csv")?;
    
    // Write header
    wtr.write_record(&[
        "Metric", "Value", "Unit"
    ])?;
    
    // Write basic stats
    wtr.write_record(&["Start Time", &report.start_time.format("%Y-%m-%d %H:%M:%S UTC").to_string(), ""])?;
    wtr.write_record(&["End Time", &report.end_time.format("%Y-%m-%d %H:%M:%S UTC").to_string(), ""])?;
    wtr.write_record(&["Duration", &format!("{:.2}", report.duration.as_secs_f64()), "seconds"])?;
    wtr.write_record(&["Input Directory", &report.input_dir.display().to_string(), ""])?;
    wtr.write_record(&["Output Directory", &report.output_dir.display().to_string(), ""])?;
    wtr.write_record(&["Total Files", &report.total_files.to_string(), "files"])?;
    wtr.write_record(&["Processed Files", &report.processed_files.to_string(), "files"])?;
    wtr.write_record(&["Failed Files", &report.failed_files.to_string(), "files"])?;
    wtr.write_record(&["Skipped Files", &report.skipped_files.to_string(), "files"])?;
    wtr.write_record(&["Original Size", &format_size(report.original_size, DECIMAL), ""])?;
    wtr.write_record(&["Compressed Size", &format_size(report.compressed_size, DECIMAL), ""])?;
    wtr.write_record(&["Space Saved", &format_size(report.original_size.saturating_sub(report.compressed_size), DECIMAL), ""])?;
    wtr.write_record(&["Compression Ratio", &format!("{:.1}%", report.compression_ratio), ""])?;
    wtr.write_record(&["Processing Speed", &format!("{:.1}", report.files_per_second), "files/sec"])?;
    wtr.write_record(&["Throughput", &format_size(report.bytes_per_second, DECIMAL), "bytes/sec"])?;
    wtr.write_record(&["Thread Count", &report.thread_count.to_string(), "threads"])?;
    wtr.write_record(&["Quality Setting", &report.quality.to_string(), ""])?;
    wtr.write_record(&["Mode", &report.mode, ""])?;
    
    // Write format statistics
    wtr.write_record(&["", "", ""])?; // Empty row
    wtr.write_record(&["Format Statistics", "", ""])?;
    for (format, count) in &report.format_stats {
        wtr.write_record(&[&format!("{} Files", format.to_uppercase()), &count.to_string(), "files"])?;
    }
    
    // Write errors if any
    if !report.errors.is_empty() {
        wtr.write_record(&["", "", ""])?; // Empty row
        wtr.write_record(&["Errors", "", ""])?;
        for (i, error) in report.errors.iter().enumerate() {
            wtr.write_record(&[&format!("Error {}", i + 1), error, ""])?;
        }
    }
    
    wtr.flush()?;
    Ok(())
}

fn generate_html_report(report: &ConversionReport) -> Result<()> {
    let html_content = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>webpify Conversion Report</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 10px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        .header {{
            background: linear-gradient(135deg, #2d3748 0%, #4a5568 100%);
            color: white;
            padding: 30px;
            text-align: center;
        }}
        .header h1 {{
            margin: 0;
            font-size: 2.5em;
            font-weight: 300;
        }}
        .header p {{
            margin: 10px 0 0 0;
            opacity: 0.8;
            font-size: 1.1em;
        }}
        .content {{
            padding: 30px;
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: #f8fafc;
            border: 1px solid #e2e8f0;
            border-radius: 8px;
            padding: 20px;
            text-align: center;
            transition: transform 0.2s;
        }}
        .stat-card:hover {{
            transform: translateY(-2px);
            box-shadow: 0 10px 20px rgba(0,0,0,0.1);
        }}
        .stat-value {{
            font-size: 2em;
            font-weight: bold;
            color: #2d3748;
            margin-bottom: 5px;
        }}
        .stat-label {{
            color: #718096;
            font-size: 0.9em;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        .section {{
            margin-bottom: 30px;
        }}
        .section h2 {{
            color: #2d3748;
            border-bottom: 2px solid #667eea;
            padding-bottom: 10px;
            margin-bottom: 20px;
        }}
        .format-table {{
            width: 100%;
            border-collapse: collapse;
            background: white;
            border-radius: 8px;
            overflow: hidden;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}
        .format-table th, .format-table td {{
            padding: 15px;
            text-align: left;
            border-bottom: 1px solid #e2e8f0;
        }}
        .format-table th {{
            background: #667eea;
            color: white;
            font-weight: 600;
        }}
        .format-table tr:hover {{
            background: #f7fafc;
        }}
        .error-list {{
            background: #fed7d7;
            border: 1px solid #fc8181;
            border-radius: 8px;
            padding: 20px;
        }}
        .error-item {{
            margin-bottom: 10px;
            padding: 10px;
            background: white;
            border-radius: 4px;
            border-left: 4px solid #e53e3e;
        }}
        .success-badge {{
            background: #68d391;
            color: white;
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 0.8em;
            font-weight: bold;
        }}
        .compression-bar {{
            width: 100%;
            height: 20px;
            background: #e2e8f0;
            border-radius: 10px;
            overflow: hidden;
            margin: 10px 0;
        }}
        .compression-fill {{
            height: 100%;
            background: linear-gradient(90deg, #48bb78, #38a169);
            width: {compression_percentage:.1}%;
            transition: width 1s ease-in-out;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 webpify</h1>
            <p>High-Performance Image Conversion Report</p>
        </div>
        
        <div class="content">
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-value">{processed_files}</div>
                    <div class="stat-label">Files Processed</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{compression_ratio:.1}%</div>
                    <div class="stat-label">Compression Ratio</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{space_saved}</div>
                    <div class="stat-label">Space Saved</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{files_per_second:.1}</div>
                    <div class="stat-label">Files/Second</div>
                </div>
            </div>
            
            <div class="section">
                <h2>📊 Conversion Overview</h2>
                <div class="compression-bar">
                    <div class="compression-fill"></div>
                </div>
                <p><strong>Compression Efficiency:</strong> {compression_ratio:.1}% space reduction achieved</p>
                
                <table class="format-table">
                    <tr>
                        <th>Metric</th>
                        <th>Value</th>
                    </tr>
                    <tr><td>Start Time</td><td>{start_time}</td></tr>
                    <tr><td>End Time</td><td>{end_time}</td></tr>
                    <tr><td>Duration</td><td>{duration:.2} seconds</td></tr>
                    <tr><td>Input Directory</td><td>{input_dir}</td></tr>
                    <tr><td>Output Directory</td><td>{output_dir}</td></tr>
                    <tr><td>Total Files Found</td><td>{total_files}</td></tr>
                    <tr><td>Successfully Processed</td><td>{processed_files} <span class="success-badge">✓</span></td></tr>
                    <tr><td>Failed</td><td>{failed_files}</td></tr>
                    <tr><td>Skipped</td><td>{skipped_files}</td></tr>
                    <tr><td>Original Size</td><td>{original_size}</td></tr>
                    <tr><td>Compressed Size</td><td>{compressed_size}</td></tr>
                    <tr><td>Processing Speed</td><td>{files_per_second:.1} files/sec</td></tr>
                    <tr><td>Throughput</td><td>{throughput}/sec</td></tr>
                    <tr><td>Thread Count</td><td>{thread_count}</td></tr>
                    <tr><td>Quality Setting</td><td>{quality}</td></tr>
                    <tr><td>Compression Mode</td><td>{mode}</td></tr>
                </table>
            </div>
            
            <div class="section">
                <h2>📁 Format Statistics</h2>
                <table class="format-table">
                    <thead>
                        <tr>
                            <th>Format</th>
                            <th>Files Processed</th>
                            <th>Percentage</th>
                        </tr>
                    </thead>
                    <tbody>
                        {format_rows}
                    </tbody>
                </table>
            </div>
            
            {error_section}
        </div>
    </div>
</body>
</html>
"#,
        compression_percentage = report.compression_ratio,
        processed_files = report.processed_files,
        compression_ratio = report.compression_ratio,
        space_saved = format_size(report.original_size.saturating_sub(report.compressed_size), DECIMAL),
        files_per_second = report.files_per_second,
        start_time = report.start_time.format("%Y-%m-%d %H:%M:%S UTC"),
        end_time = report.end_time.format("%Y-%m-%d %H:%M:%S UTC"),
        duration = report.duration.as_secs_f64(),
        input_dir = report.input_dir.display(),
        output_dir = report.output_dir.display(),
        total_files = report.total_files,
        failed_files = report.failed_files,
        skipped_files = report.skipped_files,
        original_size = format_size(report.original_size, DECIMAL),
        compressed_size = format_size(report.compressed_size, DECIMAL),
        throughput = format_size(report.bytes_per_second, DECIMAL),
        thread_count = report.thread_count,
        quality = report.quality,
        mode = report.mode,
        format_rows = generate_format_rows(&report.format_stats, report.processed_files),
        error_section = if report.errors.is_empty() {
            String::new()
        } else {
            format!(r#"
            <div class="section">
                <h2>⚠️ Errors</h2>
                <div class="error-list">
                    {}
                </div>
            </div>
            "#, report.errors.iter().enumerate()
                .map(|(i, error)| format!("<div class=\"error-item\"><strong>Error {}:</strong> {}</div>", i + 1, error))
                .collect::<Vec<_>>()
                .join(""))
        }
    );
    
    std::fs::write("conversion_report.html", html_content)?;
    Ok(())
}

fn generate_format_rows(format_stats: &HashMap<String, u64>, total_processed: u64) -> String {
    let mut rows = Vec::new();
    for (format, count) in format_stats {
        let percentage = if total_processed > 0 {
            (*count as f64 / total_processed as f64) * 100.0
        } else {
            0.0
        };
        rows.push(format!(
            "<tr><td>{}</td><td>{}</td><td>{:.1}%</td></tr>",
            format.to_uppercase(),
            count,
            percentage
        ));
    }
    rows.join("\n                        ")
}
