//! Options assembly — the one place config file, profile, and frontend
//! arguments merge into a [`ConversionOptions`].
//!
//! Precedence (low → high): built-in defaults → config file → profile → frontend args.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::{CompressionMode, ConversionOptions, ProfileConfig, ReplaceInputMode, ReportFormat};

/// Frontend-supplied values. `None` means "not specified — let config/default decide".
pub struct FrontendOptions {
    pub input_dir: PathBuf,
    pub output_dir: Option<PathBuf>,
    pub quality: Option<u8>,
    pub mode: Option<CompressionMode>,
    pub threads: Option<usize>,
    pub formats: Option<Vec<String>>,
    pub overwrite: Option<bool>,
    pub preserve_structure: Option<bool>,
    pub max_size: Option<u64>,
    pub min_size: Option<u64>,
    pub replace_input: Option<ReplaceInputMode>,
    pub reencode_webp: Option<bool>,
    pub dry_run: Option<bool>,
    pub generate_report: Option<bool>,
    pub report_format: Option<ReportFormat>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub config_file: Option<PathBuf>,
    pub profile: Option<String>,
}

/// CLI-only display settings, merged with the same precedence as conversion options.
#[derive(Debug, Clone, Copy, Default)]
pub struct CliSettings {
    pub verbose: bool,
    pub quiet: bool,
}

/// Merge defaults → config file → profile → frontend args into conversion options.
pub fn assemble(frontend: FrontendOptions) -> Result<(ConversionOptions, CliSettings)> {
    let config = match frontend.config_file.clone().or_else(discover_config_file) {
        Some(path) => Some(load_config(&path)?),
        None => None,
    };

    let profile = frontend.profile.as_deref().and_then(|name| {
        config
            .as_ref()
            .and_then(|c| c.profiles.as_ref())
            .and_then(|p| p.get(name))
    });

    if profile.is_none() && frontend.profile.is_some() {
        anyhow::bail!(
            "profile {:?} not found in any config file",
            frontend.profile.as_deref().unwrap()
        );
    }

    let mut options = ConversionOptions {
        input_dir: frontend.input_dir.clone(),
        ..ConversionOptions::default()
    };
    let mut cli = CliSettings::default();

    apply_config(&mut options, &mut cli, config.as_ref())?;
    if let Some(profile) = profile {
        apply_profile(&mut options, profile)?;
    }
    apply_frontend(&mut options, &mut cli, &frontend);

    Ok((options, cli))
}

/// Config file search order (README): explicit path, `./webpify.config.toml`,
/// `~/.config/webpify/config.toml`, `~/.config/webpify/profiles.toml`,
/// `/etc/webpify/config.toml`.
pub fn discover_config_file() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("webpify.config.toml")];
    if let Some(config_dir) = dirs::config_dir() {
        candidates.push(config_dir.join("webpify/config.toml"));
        candidates.push(config_dir.join("webpify/profiles.toml"));
    }
    candidates.push(PathBuf::from("/etc/webpify/config.toml"));
    candidates.into_iter().find(|path| path.is_file())
}

pub fn load_config(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("Invalid config file: {}", path.display()))
}

fn apply_config(
    options: &mut ConversionOptions,
    cli: &mut CliSettings,
    config: Option<&Config>,
) -> Result<()> {
    let Some(config) = config else {
        return Ok(());
    };

    if let Some(general) = &config.general {
        if options.input_dir.as_os_str().is_empty()
            && let Some(input_dir) = &general.input_dir
        {
            options.input_dir = PathBuf::from(input_dir);
        }
        if let Some(output_dir) = &general.output_dir {
            options.output_dir = Some(PathBuf::from(output_dir));
        }
        if let Some(preserve_structure) = general.preserve_structure {
            options.preserve_structure = preserve_structure;
        }
        if let Some(overwrite) = general.overwrite {
            options.overwrite = overwrite;
        }
        if let Some(threads) = general.threads {
            options.threads = normalize_threads(threads);
        }
        if let Some(replace_input) = &general.replace_input {
            options.replace_input = parse_replace(replace_input)?;
        }
        if let Some(reencode_webp) = general.reencode_webp {
            options.reencode_webp = reencode_webp;
        }
        if let Some(dry_run) = general.dry_run {
            options.dry_run = dry_run;
        }
    }

    if let Some(compression) = &config.compression {
        if let Some(quality) = compression.quality {
            options.quality = quality.clamp(0, 100);
        }
        if let Some(mode) = &compression.mode {
            options.mode = parse_mode(mode)?;
        }
    }

    if let Some(filtering) = &config.filtering {
        if let Some(formats) = &filtering.formats
            && !formats.is_empty()
        {
            options.formats = formats.iter().map(|f| f.to_lowercase()).collect();
        }
        if let Some(min_size) = filtering.min_size {
            options.min_size = min_size;
        }
        if let Some(max_size) = filtering.max_size {
            options.max_size = normalize_max_size(max_size);
        }
    }

    if let Some(output) = &config.output {
        if let Some(verbose) = output.verbose {
            cli.verbose = verbose;
        }
        if let Some(quiet) = output.quiet {
            cli.quiet = quiet;
        }
        if let Some(generate_report) = output.generate_report {
            options.generate_report = generate_report;
        }
        if let Some(report_format) = &output.report_format {
            options.report_format = parse_report_format(report_format)?;
        }
    }

    Ok(())
}

fn apply_profile(options: &mut ConversionOptions, profile: &ProfileConfig) -> Result<()> {
    if let Some(quality) = profile.quality {
        options.quality = quality.clamp(0, 100);
    }
    if let Some(mode) = &profile.mode {
        options.mode = parse_mode(mode)?;
    }
    if let Some(max_size) = profile.max_size {
        options.max_size = normalize_max_size(max_size);
    }
    if let Some(preserve_structure) = profile.preserve_structure {
        options.preserve_structure = preserve_structure;
    }
    if let Some(formats) = &profile.formats
        && !formats.is_empty()
    {
        options.formats = formats.iter().map(|f| f.to_lowercase()).collect();
    }
    if let Some(threads) = profile.threads {
        options.threads = normalize_threads(threads);
    }
    Ok(())
}

fn apply_frontend(
    options: &mut ConversionOptions,
    cli: &mut CliSettings,
    frontend: &FrontendOptions,
) {
    if !frontend.input_dir.as_os_str().is_empty() {
        options.input_dir = frontend.input_dir.clone();
    }
    if let Some(output_dir) = &frontend.output_dir {
        options.output_dir = Some(output_dir.clone());
    }
    if let Some(quality) = frontend.quality {
        options.quality = quality.clamp(0, 100);
    }
    if let Some(mode) = &frontend.mode {
        options.mode = mode.clone();
    }
    if let Some(threads) = frontend.threads {
        options.threads = normalize_threads(threads);
    }
    if let Some(formats) = &frontend.formats
        && !formats.is_empty()
    {
        options.formats = formats.clone();
    }
    if let Some(overwrite) = frontend.overwrite {
        options.overwrite = overwrite;
    }
    if let Some(preserve_structure) = frontend.preserve_structure {
        options.preserve_structure = preserve_structure;
    }
    if let Some(max_size) = frontend.max_size {
        options.max_size = normalize_max_size(max_size);
    }
    if let Some(min_size) = frontend.min_size {
        options.min_size = min_size;
    }
    if let Some(replace_input) = &frontend.replace_input {
        options.replace_input = replace_input.clone();
    }
    if let Some(reencode_webp) = frontend.reencode_webp {
        options.reencode_webp = reencode_webp;
    }
    if let Some(dry_run) = frontend.dry_run {
        options.dry_run = dry_run;
    }
    if let Some(generate_report) = frontend.generate_report {
        options.generate_report = generate_report;
    }
    if let Some(report_format) = &frontend.report_format {
        options.report_format = report_format.clone();
    }
    if let Some(verbose) = frontend.verbose {
        cli.verbose = verbose;
    }
    if let Some(quiet) = frontend.quiet {
        cli.quiet = quiet;
    }
}

/// `0` means "unlimited" in config files (example.config.toml convention).
fn normalize_max_size(max_size: u64) -> Option<u64> {
    (max_size > 0).then_some(max_size)
}

/// `0` means "all available threads" in profiles.toml.
fn normalize_threads(threads: usize) -> Option<usize> {
    (threads > 0).then_some(threads)
}

fn parse_mode(value: &str) -> Result<CompressionMode> {
    match value.trim().to_lowercase().as_str() {
        "lossless" => Ok(CompressionMode::Lossless),
        "lossy" => Ok(CompressionMode::Lossy),
        "auto" => Ok(CompressionMode::Auto),
        other => {
            anyhow::bail!("unknown compression mode {other:?} (expected lossless, lossy, or auto)")
        }
    }
}

fn parse_replace(value: &str) -> Result<ReplaceInputMode> {
    match value.trim().to_lowercase().as_str() {
        "off" => Ok(ReplaceInputMode::Off),
        "recycle" => Ok(ReplaceInputMode::Recycle),
        "delete" => Ok(ReplaceInputMode::Delete),
        other => {
            anyhow::bail!("unknown replace_input {other:?} (expected off, recycle, or delete)")
        }
    }
}

fn parse_report_format(value: &str) -> Result<ReportFormat> {
    match value.trim().to_lowercase().as_str() {
        "json" => Ok(ReportFormat::Json),
        "csv" => Ok(ReportFormat::Csv),
        "html" => Ok(ReportFormat::Html),
        other => anyhow::bail!("unknown report format {other:?} (expected json, csv, or html)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SAMPLE: &str = r#"
[general]
output_dir = "./webp_output"
threads = 4
replace_input = "recycle"

[compression]
quality = 85
mode = "auto"

[filtering]
formats = ["jpg", "png"]
min_size = 2
max_size = 10

[output]
verbose = true
generate_report = true
report_format = "csv"

[profiles.web]
quality = 90
mode = "lossless"
threads = 8
"#;

    fn base(input_dir: &str) -> FrontendOptions {
        FrontendOptions {
            input_dir: PathBuf::from(input_dir),
            output_dir: None,
            quality: None,
            mode: None,
            threads: None,
            formats: None,
            overwrite: None,
            preserve_structure: None,
            max_size: None,
            min_size: None,
            replace_input: None,
            reencode_webp: None,
            dry_run: None,
            generate_report: None,
            report_format: None,
            verbose: None,
            quiet: None,
            config_file: None,
            profile: None,
        }
    }

    fn write_sample() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, SAMPLE).unwrap();
        (dir, path)
    }

    #[test]
    fn no_config_means_defaults() {
        let (options, cli) = assemble(base("/in")).unwrap();
        assert_eq!(options.quality, 80);
        assert_eq!(options.mode, CompressionMode::Lossless);
        assert_eq!(options.threads, None);
        assert_eq!(options.replace_input, ReplaceInputMode::Off);
        assert_eq!(options.formats.len(), 7);
        assert!(!options.generate_report);
        assert_eq!(options.report_format, ReportFormat::Json);
        assert!(!cli.verbose && !cli.quiet);
    }

    #[test]
    fn config_applies_after_defaults() {
        let (_dir, path) = write_sample();
        let mut fe = base("/in");
        fe.config_file = Some(path);
        let (options, cli) = assemble(fe).unwrap();
        assert_eq!(options.quality, 85);
        assert_eq!(options.mode, CompressionMode::Auto);
        assert_eq!(options.threads, Some(4));
        assert_eq!(options.replace_input, ReplaceInputMode::Recycle);
        assert_eq!(options.formats, vec!["jpg".to_string(), "png".to_string()]);
        assert_eq!(options.min_size, 2);
        assert_eq!(options.max_size, Some(10));
        assert!(options.generate_report);
        assert_eq!(options.report_format, ReportFormat::Csv);
        assert!(cli.verbose);
    }

    #[test]
    fn frontend_overrides_config() {
        let (_dir, path) = write_sample();
        let mut fe = base("/in");
        fe.config_file = Some(path);
        fe.quality = Some(70);
        fe.threads = Some(2);
        fe.max_size = Some(0); // CLI 0 = unlimited
        fe.quiet = Some(true);
        let (options, cli) = assemble(fe).unwrap();
        assert_eq!(options.quality, 70);
        assert_eq!(options.threads, Some(2));
        assert_eq!(options.max_size, None);
        assert!(cli.quiet);
    }

    #[test]
    fn profile_overrides_config() {
        let (_dir, path) = write_sample();
        let mut fe = base("/in");
        fe.config_file = Some(path);
        fe.profile = Some("web".to_string());
        let (options, _cli) = assemble(fe).unwrap();
        assert_eq!(options.quality, 90);
        assert_eq!(options.mode, CompressionMode::Lossless);
        assert_eq!(options.threads, Some(8));
        // profile does not touch filtering: config values survive
        assert_eq!(options.min_size, 2);
    }

    #[test]
    fn unknown_profile_is_an_error() {
        let (_dir, path) = write_sample();
        let mut fe = base("/in");
        fe.config_file = Some(path);
        fe.profile = Some("nope".to_string());
        assert!(assemble(fe).is_err());
    }

    #[test]
    fn unknown_mode_in_config_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("config.toml");
        fs::write(&bad, "[compression]\nmode = \"bogus\"\n").unwrap();
        let mut fe = base("/in");
        fe.config_file = Some(bad);
        assert!(assemble(fe).is_err());
    }

    #[test]
    fn zero_values_mean_unlimited_or_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "[filtering]\nmax_size = 0\nmin_size = 0\n\n[general]\nthreads = 0\n",
        )
        .unwrap();
        let mut fe = base("/in");
        fe.config_file = Some(path);
        let (options, _cli) = assemble(fe).unwrap();
        assert_eq!(options.max_size, None);
        assert_eq!(options.threads, None);
        assert_eq!(options.min_size, 0);
    }
}
