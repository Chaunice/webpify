# webpify — Domain Context

Batch image → WebP converter. Rust library + two frontends: CLI binary (`main.rs`) and GUI binary (`gui.rs`, egui).

## Glossary

- **OptionsAssembly** — module (`src/options.rs`). The one place config file, profile, and frontend arguments merge into `ConversionOptions`. Precedence (low→high): built-in defaults → config file → profile → frontend args. `assemble(FrontendOptions) -> (ConversionOptions, CliSettings)`.
- **FileDiscovery** — module (`src/discovery.rs`). The one place that decides what counts as a convertable file: extension whitelist, header validation, min/max size, webp-skip rule. Both frontends share it; GUI adds presentation caps (`max_files` 100, `max_depth` 5). Core and GUI preview therefore cannot disagree.
- **ProgressReporting** — seam (`src/progress.rs`), `ProgressReporter` trait. Core drives start/finish and per-file success/error events through it; two adapters: `ConsoleProgressReporter` (CLI) and `ThreadSafeGuiProgressReporter` (GUI, feeds the log panel). Summary counters still flow via `update_progress`.
- **ImageConverter** — module (`src/converter.rs`). One encode per image; owns the mode selection heuristics (extension + color-complexity sampling) and `estimate_output_size`, the single size estimator shared by dry-run and GUI preview.
- **ConversionReport** — data struct (`src/lib.rs`). Built from one skeleton (`Default` + core's `base_report`); run outcomes fill counters. Empty runs return a clean report — callers surface the "no files found" message themselves.
- **ConversionStats** — module (`src/stats.rs`). Thread-safe counters aggregated during a run; feeds the report. No retry/ETA surface — retries were never implemented.

## Decisions

- Config files are a CLI workflow; the GUI form exposes every option directly and has no config-file fields.
- `prescan` was a no-op option (streaming scan never existed) — removed entirely (flag, config key, GUI checkbox) rather than kept as dead surface.
- `0` values in config files mean "unlimited" (`max_size`) or "all cores" (`threads`), per `example.config.toml`/`profiles.toml` convention.
- Unknown profile names and invalid config values error loudly — never silently ignored.
