# ADR-0001: One options assembly module; config files are a CLI workflow

- Status: accepted
- Date: 2026-08-14
- Related: `src/options.rs`, `CONTEXT.md` (OptionsAssembly)

## Context

Config file support was documented in the README (auto-discovery across five
locations, profile precedence, "CLI arguments always take precedence over
config values") but had no caller: the CLI parsed `--config`/`--profile` and
ignored them, and the GUI had config-file fields that never loaded. Options
were assembled by hand in two places through a 15-method builder.

## Decision

1. All option assembly goes through one module (`options::assemble`):
   built-in defaults → config file → profile → frontend args, with a single
   precedence rule. Both frontends call it.
2. Config files and profiles are a CLI workflow. The GUI form exposes every
   option directly and has no config-file/profile fields.
3. Unknown profile names and invalid config values are errors, not silent
   no-ops. `0` values mean "unlimited" (`max_size`) or "all cores"
   (`threads`).
4. The `ConversionOptions` builder was deleted; the struct is plain data
   with `Default`.

## Consequences

- Precedence bugs now have one home instead of two hand-rolled chains.
- Config discovery order: explicit `--config`, `./webpify.config.toml`,
  `~/.config/webpify/config.toml`, `~/.config/webpify/profiles.toml`,
  `/etc/webpify/config.toml`.
- GUI users configure through the form; a config file cannot pre-fill the
  GUI form. Revisit if GUI repeatability becomes a requested feature.
