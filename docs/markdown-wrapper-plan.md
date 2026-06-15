# Plan: Markdown wrapper

## Summary

Add a markdown wrapper tool that wraps markdown prose to a configurable display
width. The default width is 80 characters, and `-n {NUMBER}` overrides that
default for a run. The tool accepts one or more paths; file paths are wrapped in
place, and directory paths are traversed recursively for `*.md` files.

## README changes

- Add a `Wrap markdown prose` section describing the tool and its behavior.
- Document CLI examples for the future binaries:
  - `bin/go-wrap README.md`
  - `bin/rust-wrap -n 100 docs/`
- Extend the build and project-layout sections to mention `go-wrap` and
  `rust-wrap` alongside the existing aligner binaries.
- Describe that the wrapper will follow the same Go/Rust parity model and shared
  fixture approach used by the table aligner.

## Behavior contract

- Wrap prose paragraphs only.
- Leave fenced code blocks, tables, headings, lists, blockquotes, and front
  matter untouched.
- Preserve paragraph boundaries, original line endings, and trailing-newline
  state.
- For directory inputs, recursively process only `*.md` files.
- Rewrite files only when content changes.
- Print the path of every changed file, one per line, matching the aligner CLI
  reporting pattern.

## Future interface

- CLI names: `go-wrap` and `rust-wrap`.
- CLI shape: `[-n NUMBER] <path> [path...]`.
- Future library shape should mirror the aligner:
  - a pure string wrapper function for in-memory formatting
  - a path/directory formatter that rewrites changed files and returns changed
    paths

## Future test scenarios

- Default 80-column prose wrapping.
- Custom width with `-n`.
- Recursive directory traversal over nested `*.md` files.
- Non-markdown files ignored.
- Code fences, tables, headings, lists, blockquotes, and front matter unchanged.
- Idempotency on already wrapped markdown.
- Line-ending and trailing-newline preservation.

## Assumptions

- This plan is documentation-only; it does not add implementation files, tests,
  Makefile changes, or README edits.
- Wrapping applies to prose paragraphs only.
- The wrapper will eventually be implemented in both Go and Rust.
