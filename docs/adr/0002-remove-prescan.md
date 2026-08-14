# ADR-0002: Remove the prescan option (it was a fake seam)

- Status: accepted
- Date: 2026-08-14
- Related: `src/discovery.rs`, `CONTEXT.md` (FileDiscovery)

## Context

`ConversionOptions` exposed `prescan: bool` with two code paths:
`scan_input_files` (real) and `scan_files_streaming` (a copy of the first,
marked "could be optimized for very large directories"). The option existed
in the CLI, config format, README, and GUI — but the streaming variant
never existed, so the flag was a documented no-op that implied behavior the
tool did not have.

## Decision

Delete the option and the seam entirely: CLI flag, config key, GUI
checkbox, README mentions, and the dead `scan_files_streaming` function.
File discovery is one implementation (`discover_files`), always streaming
via `WalkDir`, with the same filter rule set for core and GUI preview.

## Consequences

- Removing the flag is a breaking CLI change (0.4.0; pre-1.0, acceptable).
  Config files containing `prescan` still parse — unknown TOML keys are
  ignored.
- If large-directory streaming ever needs tuning, it is a change inside
  `discovery.rs`, not a new option.
- A future "skip the header scan for speed" mode would be a real second
  implementation and a new seam — worth an ADR at that point.
