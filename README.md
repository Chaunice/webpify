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

USAGE:
    webpify [OPTIONS] --input <DIR>

OPTIONS:
    -i, --input <DIR>              Input directory path
    -o, --output <DIR>             Output directory path (defaults to input_dir/webp_output)
    -q, --quality <QUALITY>        WebP compression quality (0-100) [default: 80]
    -t, --threads <NUM>            Number of parallel threads (defaults to CPU core count for I/O optimization)
    -m, --mode <MODE>              Compression mode [default: lossless]
                                   [possible values: lossless, lossy, auto]
        --formats <FORMATS>        Supported input formats (defaults to common formats)
                                   [default: jpg jpeg png gif bmp tiff webp]
        --overwrite                Overwrite existing files
        --preserve-structure       Preserve original directory structure
        --max-size <SIZE>          Maximum file size limit (MB)
        --min-size <SIZE>          Minimum file size limit (KB) [default: 1]
        --prescan                  Enable pre-processing scan
        --performance <PERFORMANCE> Performance mode: fast, balanced, quality [default: balanced]
    -v, --verbose                  Verbose output mode
        --quiet                    Quiet mode (results only)
        --report                   Generate conversion report
        --report-format <REPORT_FORMAT> Report output format [default: json] [possible values: json, csv, html]
    -c, --config <FILE>            Configuration file path
        --ultra-fast               Ultra-fast mode (trades quality for speed)
    -h, --help                     Print help
    -V, --version                  Print version
```

## 🔧 Advanced Configuration

### Performance Tuning

```bash
# SSD-optimized (high concurrency)
webpify -i ./images -t 24 --prescan false

# HDD-optimized (low concurrency)
webpify -i ./images -t 4 --min-size 10

# Memory-constrained environment
webpify -i ./images -t 2 --max-size 10
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [image-rs](https://github.com/image-rs/image) - Powerful Rust image processing library
- [webp](https://crates.io/crates/webp) - WebP encoding support
- [rayon](https://github.com/rayon-rs/rayon) - Data parallelism framework
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
