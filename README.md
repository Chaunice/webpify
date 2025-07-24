# webpify

**webpify** is a high-performance batch image to WebP converter designed for large-scale image processing tasks. Efficiently converts various image formats to WebP with significant space savings.

## ✨ Features

- **Extreme Performance**: Multi-threaded parallel processing leveraging full CPU power
- **Real-time Monitoring**: Beautiful progress bars with live statistics
- **Smart Compression**: Supports lossy/lossless/auto modes with intelligent strategy selection
- **Significant Space Savings**: WebP format saves 20-80% storage space
- **Deep Scanning**: Recursive directory scanning with nested folder support
- **Comprehensive Reports**: Generate JSON/CSV/HTML conversion reports
- **Rock Solid**: Memory-safe, robust error handling, supports large file processing
- **User Friendly**: Intuitive CLI with rich configuration options

## 🚀 Quick Start

### Installation

Download the latest release from the release page or build from source.

```bash
# Build from source
git clone https://github.com/chaunice/webpify.git
cd webpify
cargo build --release
```

### Basic Usage

```bash
# Convert single directory
webpify -i ./photos -o ./webp_output

# High-quality lossy compression (recommended)
webpify -i ./photos -q 90 -m lossy

# Lossless compression (suitable for PNG, graphics)
webpify -i ./graphics -m lossless

# Auto mode (intelligent compression strategy selection)
webpify -i ./mixed_images -m auto

# High-performance mode (16 threads parallel processing)
webpify -i ./large_dataset -t 16 --quiet
```

## 📖 Command Line Options

```text
                    __                        ___             
                   /\ \                __   /'___\            
 __  __  __     __ \ \ \____   _____  /\_\ /\ \__/  __  __    
/\ \/\ \/\ \  /'__`\\ \ '__`\ /\ '__`\\/\ \\ \ ,__\/\ \/\ \   
\ \ \_/ \_/ \/\  __/ \ \ \L\ \\ \ \L\ \\ \ \\ \ \_/\ \ \_\ \  
 \ \___x___/'\ \____\ \ \_,__/ \ \ ,__/ \ \_\\ \_\  \/`____ \ 
  \/__//__/   \/____/  \/___/   \ \ \/   \/_/ \/_/   `/___/> \
                                 \ \_\                  /\___/
                                  \/_/                  \/__/ 

webpify - High-performance batch WebP converter

Usage: webpify [OPTIONS] --input <DIR>

Options:
  -i, --input <DIR>                    Input directory path
  -o, --output <DIR>                   Output directory path (defaults to input_dir/webp_output)
  -q, --quality <QUALITY>              WebP compression quality (0-100) [default: 80]
  -t, --threads <NUM>                  Number of parallel threads (defaults to CPU core count for I/O optimization)
  -m, --mode <MODE>                    Compression mode [default: lossless] [possible values: lossless, lossy, auto]
      --formats <FORMATS>              Supported input formats (defaults to common formats) [default: jpg jpeg png gif bmp tiff webp]
      --overwrite                      Overwrite existing files
      --preserve-structure             Preserve original directory structure
      --max-size <SIZE>                Maximum file size limit (MB)
      --min-size <SIZE>                Minimum file size limit (KB) [default: 1]
      --prescan                        Enable pre-processing scan
  -v, --verbose                        Verbose output mode
      --quiet                          Quiet mode (results only)
      --report                         Generate conversion report
      --report-format <REPORT_FORMAT>  Report output format [default: json] [possible values: json, csv, html]
  -c, --config <FILE>                  Configuration file path
      --replace-input <REPLACE_INPUT>  How to handle input files after successful conversion [off: keep, recycle: move to recycle bin, delete: permanently delete] [default: off] [possible values: off, recycle, delete]
      --reencode-webp                  Force re-encoding of WebP files (by default, .webp files are skipped)
      --dry-run                        Dry run mode - preview operations without making changes
      --quality-metrics               Enable quality metrics calculation (SSIM/PSNR)
      --profile <PROFILE>              Use a predefined configuration profile
  -h, --help                           Print help (see more with '--help')
  -V, --version                        Print version
```

## 🔧 Advanced Configuration

### Performance Tuning

```bash
# High concurrency (SSD or fast storage, accurate progress)
# (Prescan is enabled by default and recommended for SSDs)
webpify -i ./images -t 24

# Low concurrency (HDD or slow storage, reduce random seeks)
# (To disable prescan, set prescan = false in the config file)
webpify -i ./images -t 4 --min-size 10

# Memory-constrained environment
webpify -i ./images -t 2 --max-size 10

# Preview mode (dry run) - see what would be converted without making changes
webpify -i ./images --dry-run --verbose

# Use predefined profiles for common scenarios
webpify -i ./images --profile web
webpify -i ./images --profile print
webpify -i ./images --profile archive
```

> Note:
>
> - The `--prescan` flag is a boolean switch (enable only). To disable prescan, set `prescan = false` in your config file.
> - Prescan is enabled by default and is recommended for SSDs and most use cases. Disabling prescan may help reduce startup time and memory usage for very large datasets on slow HDDs, but progress reporting will be less accurate.

## 🛠 Example Configuration File

webpify supports TOML config files for advanced and repeatable setups. The tool will automatically search for a config file in these locations (in order):

1. Path specified by `--config <FILE>`
2. `./webpify.config.toml` (current directory)
3. `~/.config/webpify/config.toml` (Linux/macOS user config)
4. `%APPDATA%\webpify\config.toml` (Windows user config)
5. `/etc/webpify/config.toml` (system-wide, non-Windows)

The first config file found will be loaded. CLI arguments always take precedence over config values.

### `example.config.toml`

```toml
[general]
input_dir = "./images"
output_dir = "./webp_output"
preserve_structure = true
overwrite = false
threads = 8
prescan = true
replace_input = "off" # off, recycle, delete
reencode_webp = false
dry_run = false # Enable preview mode

[compression]
quality = 85
mode = "auto" # lossless, lossy, auto

[filtering]
formats = ["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"]
min_size = 1
max_size = 0

[output]
verbose = true
quiet = false
generate_report = true
report_format = "json" # json, csv, html
```

See `example.config.toml` in the repository for a full reference and comments.

## 📋 Configuration Profiles

webpify includes predefined configuration profiles for common use cases. Profiles provide optimized settings for different scenarios:

### Available Profiles

- **`web`** - Web-optimized images with good compression (quality: 85, auto mode)
- **`print`** - High-quality images suitable for printing (quality: 95, lossless)
- **`archive`** - Maximum compression for archival storage (quality: 70, lossy)
- **`fast`** - Fast processing with reasonable quality (quality: 80, lossy)
- **`lossless`** - Perfect quality preservation (quality: 100, lossless)

### Using Profiles

```bash
# Use web profile for website images
webpify -i ./photos --profile web

# Use print profile for high-quality output
webpify -i ./artwork --profile print

# Use archive profile for long-term storage
webpify -i ./backup --profile archive
```

Profiles are loaded from `profiles.toml` files in standard locations:
1. `./profiles.toml` (current directory)
2. Next to your config file (if using `--config`)
3. `~/.config/webpify/profiles.toml` (Linux/macOS)
4. `%APPDATA%\webpify\profiles.toml` (Windows)

See `profiles.toml` in the repository for profile definitions and customization options.

## 📝 TODO

- [ ] Add support for additional image formats.
- [ ] Consider adding AVIF output support.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [image-rs](https://github.com/image-rs/image) - Powerful Rust image processing library
- [webp](https://crates.io/crates/webp) - WebP encoding support
- [rayon](https://github.com/rayon-rs/rayon) - Data parallelism framework
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
