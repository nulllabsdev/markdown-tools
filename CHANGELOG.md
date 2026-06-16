# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Release process based on annotated git tags.

### Changed
- Successful `gomd` and `rustmd` commands now prepend a `build ...` line before
  normal stdout and keep a trailing blank line.
- `gomd -v` and `rustmd -v` now print that same `build ...` line and exit.

## [0.1.0] - 2026-06-16

### Added
- Markdown table alignment in Go and Rust.
- Markdown prose wrapping in Go and Rust.
- ASCII graph alignment in Go and Rust.
- Unified `gomd` and `rustmd` binaries with `align`, `wrap`, `graph`, and `all`.
