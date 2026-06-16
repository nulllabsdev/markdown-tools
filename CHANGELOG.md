# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-16

Initial release: three markdown formatting tools, implemented in both Go and
Rust and verified to produce byte-identical output.

### Added

- **Table aligner (`align`)** — aligns the columns of GitHub-style pipe tables to
  the minimal width needed. Display-width aware (CJK and emoji count as 2,
  combining marks as 0); honours `:--` / `:-:` / `--:` alignment markers
  (defaulting to left); minimum column width of 3; preserves escaped pipes
  (`\|`); pads ragged rows; leaves fenced code blocks untouched (including info
  strings, rejecting a backtick inside a backtick-fence info string and ignoring
  fences indented four or more columns); preserves LF/CRLF and mixed line
  endings; idempotent.
- **Prose wrapper (`wrap`)** — greedily reflows paragraphs to a display width
  (default 80, `-n N` to override) without ever splitting a word. Leaves headings
  (ATX and setext), lists, blockquotes, tables, fenced and indented code,
  thematic breaks, HTML blocks, and YAML front matter untouched; preserves line
  endings and trailing-newline state; idempotent.
- **ASCII graph aligner (`graph`)** — re-renders ASCII flowcharts inside fenced
  ` ```text ` blocks to a canonical centered form: 30-character inner boxes with
  centered labels, box-rows centered within 80 columns, structural connectors
  re-centered beneath the boxes, and connector text annotations preserved. Only
  graph blocks are touched; non-graph ` ```text ` blocks, other fences, and
  unfenced markdown pass through byte-for-byte; idempotent.
- **Unified CLIs `gomd` (Go) and `rustmd` (Rust)** — one binary per language with
  `align`, `wrap`, `graph`, and `all` subcommands (`all` runs align, then wrap,
  then graph). With no path each reads stdin and writes stdout; given one or more
  paths it rewrites them in place (directories are walked recursively for `*.md`)
  and prints every changed file path, one per line.
- **Library API** — mirrored in the Go package `mdtable` and the Rust crate
  `markdown_tools`: `format_str` / `format_directory`, `wrap_str` /
  `wrap_directory`, and `align_graph_str` / `align_graph_directory`.
- **Version reporting** — `gomd -v` and `rustmd -v` report the version from
  `git describe` (embedded at build time in Rust via `build.rs`), and successful
  commands print a `build <version>` line. Releases are cut from annotated `v*`
  git tags via `scripts/release.sh`.
- **Cross-language parity** — a shared `testdata` fixture suite drives both
  implementations so their output is provably byte-for-byte identical.
- **Continuous integration** — GitHub Actions builds, formats, lints, and tests
  both the Go and Rust implementations on every push and pull request.

## Links

- [0.1.0 release](https://github.com/nulllabsdev/markdown-tools/releases/tag/v0.1.0)
- [Unreleased changes](https://github.com/nulllabsdev/markdown-tools/compare/v0.1.0...HEAD)
