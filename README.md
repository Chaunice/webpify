# webpify

A batch image to WebP/AVIF converter written in Rust. Converts directories
of images in parallel with configurable compression modes, output
formatting, and report generation.

## Features

- Parallel conversion with configurable thread count (defaults to all cores)
- Lossy, lossless, and auto compression modes (auto picks per image based on
  content analysis)
- WebP or AVIF output
- Recursive directory scanning with optional structure preservation
- Input filtering by format, minimum and maximum file size
- Dry-run mode to preview what would be converted
- Optional JSON/CSV/HTML reports
- TOML configuration files and reusable profiles
- Recycle-bin or permanent deletion of source files after conversion

Supported input formats: JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, TGA, PNM,
QOI, HDR, DDS, EXR, Farbfeld.

## Installation

Build from source with a Rust toolchain (edition 2024):

```bash
git clone https://github.com/Chaunice/webpify.git
cd webpify
cargo build --release
```

The binary is `target/release/webpify`. A GUI build is available with
`cargo build --release --features gui` (binary `webpify-gui`).

## Usage

```bash
# Convert a directory into ./webp_output
webpify -i ./photos

# Explicit output directory
webpify -i ./photos -o ./webp_output

# Lossy compression at quality 90
webpify -i ./photos -q 90 -m lossy

# AVIF output (lossy or auto mode only)
webpify -i ./photos --output-format avif

# 4 threads, skip files smaller than 10 KB
webpify -i ./photos -t 4 --min-size 10

# Dry run: list what would be converted without writing anything
webpify -i ./photos --dry-run --verbose

# Predefined profile (see profiles below)
webpify -i ./photos --profile web
```

### Options

```
Usage: webpify [OPTIONS] --input <DIR>

Options:
  -i, --input <DIR>                    Input directory path
  -o, --output <DIR>                   Output directory (default: <input>/webp_output)
  -q, --quality <QUALITY>              Compression quality (0-100, default: 80)
  -t, --threads <NUM>                  Parallel threads (default: CPU core count)
  -m, --mode <MODE>                    Compression mode: lossless, lossy, auto
      --formats <FORMATS>              Comma-separated input formats (default: all supported)
      --overwrite                      Overwrite existing output files
      --preserve-structure             Preserve directory structure in output
      --max-size <SIZE>                Skip files larger than SIZE MB
      --min-size <SIZE>                Skip files smaller than SIZE KB (default: 1)
      --output-format <FORMAT>         Output format: webp, avif (default: webp)
      --report                         Generate a conversion report
      --report-format <FORMAT>         Report format: json, csv, html (default: json)
      --replace-input <MODE>           Source handling after conversion: off, recycle, delete
      --reencode-webp                  Also convert existing .webp files
      --dry-run                        Preview operations without making changes
  -c, --config <FILE>                  Configuration file path
      --profile <PROFILE>              Use a predefined configuration profile
  -v, --verbose                        Verbose logging
      --quiet                          Suppress progress output
  -h, --help                           Print help
  -V, --version                        Print version
```

## Configuration

Configuration is TOML-based and optional. The tool looks for a config file
in this order and loads the first one found:

1. Path given with `--config`
2. `./webpify.config.toml` (current directory)
3. `~/.config/webpify/config.toml` (user config)
4. `~/.config/webpify/profiles.toml` (standalone profiles file)
5. `/etc/webpify/config.toml` (system-wide, non-Windows)

Precedence: CLI arguments > profile values > config file values > defaults.

```toml
[general]
output_dir = "./webp_output"
threads = 8
replace_input = "off"   # off, recycle, delete
reencode_webp = false
dry_run = false

[compression]
quality = 85
mode = "auto"           # lossless, lossy, auto

[filtering]
formats = ["jpg", "png", "webp"]
min_size = 1            # KB, 0 disables
max_size = 0            # MB, 0 means unlimited

[output]
generate_report = true
report_format = "json"  # json, csv, html
format = "webp"         # webp, avif
```

See [example.config.toml](example.config.toml) for a commented reference.

### Profiles

Profiles bundle common settings under a name. They live in the `profiles`
table of a config file, or in a standalone `profiles.toml` (see discovery
order above). `profiles.toml` in this repository is a sample you can copy
to `~/.config/webpify/`.

```bash
webpify -i ./photos --profile web
webpify -i ./photos --profile print
webpify -i ./artwork --profile archive
```

## Reports

With `--report`, a summary is written to the working directory as
`webpify_report.json`, `webpify_report.csv`, or `webpify_report.html`,
containing per-run statistics: file counts, sizes, compression ratio,
throughput, and per-file errors.

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- [image-rs](https://github.com/image-rs/image) — image decoding and AVIF encoding
- [webp](https://crates.io/crates/webp) — WebP encoding
- [rayon](https://github.com/rayon-rs/rayon) — data parallelism
- [clap](https://github.com/clap-rs/clap) — CLI parsing
