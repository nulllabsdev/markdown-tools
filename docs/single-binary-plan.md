# Plan: unify the CLIs into `gomd` and `rustmd`

## Summary

Replace the six per-tool binaries (`go-align`, `go-wrap`, `go-align-graph` and
their Rust counterparts) with **one binary per language** — `gomd` (Go) and
`rustmd` (Rust) — that selects a feature via subcommands. Alongside, restructure
the Rust crate so **every feature is its own module** and `lib.rs` is just
module wiring plus shared helpers.

This document is a planning artifact only. It does not add implementation,
tests, Makefile targets, or README changes.

## CLI design

Shape (identical for both binaries): `gomd <subcommand> [flags] [paths…]`.

- Subcommands: `align` (table aligner), `wrap` (prose wrapper), `graph`
  (ASCII-graph aligner), and `all` (run all three).
- No paths after the subcommand → read markdown from stdin, write the result to
  stdout.
- One or more paths → process each in place; a directory is walked recursively
  for `*.md`; a file is rewritten only when its content changes; every changed
  file path is printed, one per line.
- `wrap` and `all` accept `-n N` (default 80) for the wrap width; `align` and
  `graph` take no flags.
- Missing or unknown subcommand → usage message on stderr, exit code 1.

`all` composes the three in a fixed order **align → wrap → graph**:

- stdin/file: `align_graph(wrap(align_tables(text), n))`.
- directory: run the three directory walkers in sequence and print the
  deduplicated, sorted union of changed paths. The tools are orthogonal — the
  table aligner touches pipe tables, the wrapper touches prose, the graph aligner
  touches fenced `text` graphs — so composition is well-defined and idempotent.

## Rust module layout

- `mod common` — shared helpers used by more than one feature: `LogicalLine`,
  `split_lines`, `disp_width`, `opening_fence_token`, `is_closing_fence`,
  `indented_too_far`, `is_pipe_row`, `is_markdown` (all `pub(crate)`).
- `mod table` — the table-aligner core currently living in `lib.rs`: `format_str`,
  `format_directory`, and the private table parsing/rendering (`Align`,
  `MIN_COL_WIDTH`, `format_table`, `split_cells`, separator/row rendering, and the
  aligner's directory walker).
- `mod wrap`, `mod align_graph` — unchanged logic; their `use crate::{…}` imports
  repoint to `use crate::common::{…}`.
- `lib.rs` — only module declarations and `pub use` re-exports of the public API.

The public function names are unchanged (`format_str`/`format_directory`,
`wrap_str`/`wrap_directory`, `align_graph_str`/`align_graph_directory`), so the
existing integration tests continue to pass without edits.

The Go package `mdtable` keeps its per-feature files (`align.go`, `wrap.go`,
`align_graph.go`); the per-feature-module change is Rust-only.

## Files

- Create: `rust/src/common.rs`, `rust/src/table.rs`, `rust/src/bin/rustmd.rs`,
  `go/cmd/gomd/main.go`.
- Remove: `rust/src/bin/rust-align.rs`, `rust-wrap.rs`, `rust-align-graph.rs`;
  `go/cmd/go-align/`, `go/cmd/go-wrap/`, `go/cmd/go-align-graph/`.
- Modify: `rust/src/lib.rs` (facade), `rust/src/wrap.rs` and `align_graph.rs`
  (imports), `rust/Cargo.toml` (replace three `[[bin]]` entries with one named
  `rustmd`), `Makefile` (replace the six build targets with `gomd` and `rustmd`;
  `all: gomd rustmd`; `test` unchanged), `.gitignore` (stray-binary guard →
  `/go/gomd`), `README.md` (CLI section and project layout for the two binaries
  and their subcommands).

## Future test scenarios

- Each subcommand reproduces the output of the corresponding old per-tool binary.
- `gomd` and `rustmd` produce byte-identical output per subcommand over the shared
  fixtures.
- `all` applies all three transforms; re-running is idempotent; a directory run
  reports a deduplicated, sorted union of changed paths.
- Missing/unknown subcommand exits non-zero with a usage message.
- `go test ./...` and `cargo test` remain green (public API unchanged); `make test`
  passes.

## Assumptions

- The feature is selected by subcommands (`align` / `wrap` / `graph`), plus an
  `all` subcommand — not by boolean flags.
- `all` runs in the order align → wrap → graph; the tools are orthogonal, so the
  order does not change results on normal input.
- Library function names stay the same, so tests need no changes.
- The Rust module split does not deduplicate the three per-feature directory
  walkers (pre-existing duplication); that is left for a later cleanup.
