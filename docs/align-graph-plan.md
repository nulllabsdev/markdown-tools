# Plan: align-graph

## Summary

`align-graph` is a future ASCII graph alignment tool for markdown. It reformats
ASCII flowcharts inside fenced `text` code blocks so boxes, labels, and branches
are consistently centered and readable in raw markdown.

This document is a planning artifact only. It does not add implementation,
tests, Makefile targets, or README changes.

## Formatting contract

- Each box has exactly 30 characters of inner width.
- Text inside a box is centered within that 30-character inner width.
- A single box, or a row of multiple boxes, is centered as a group within columns
  `0..80`.
- Connector lines are adjusted so vertical and branching connections remain
  visually centered beneath or between boxes.
- Already aligned graphs should be idempotent.
- Non-graph fenced `text` blocks are left untouched.

## Scope

- Process only fenced `text` code blocks.
- Leave non-`text` code fences untouched.
- Leave all unfenced markdown untouched.
- Use the existing Bodul-style flowchart fixtures as representative source
  examples.

## Future interface

- CLI entrypoints:
  - `gomd graph`
  - `rustmd graph`
- CLI behavior should match the existing markdown tools:
  - no path arguments after `graph`: read markdown from stdin and write aligned
    markdown to stdout
  - one or more file paths: rewrite changed files in place
  - directory paths: recurse through `*.md` files
  - print each changed file path, one per line
- Future library API should mirror the existing project model:
  - `align_graph_str` for pure in-memory string processing
  - `align_graph_directory` for recursive in-place path processing
  - changed-path returns should match existing directory formatter APIs

## Examples

Single centered box with 30 characters of inner width:

```text
                        +------------------------------+
                        |            Start             |
                        +------------------------------+
```

Centered two-box row:

```text
       +------------------------------+  +------------------------------+
       |       Discover sitemap       |  |       Discover catalog       |
       |         files and URLs       |  |         leaves and URLs      |
       +------------------------------+  +------------------------------+
```

Centered branch connector:

```text
                      +----------------+----------------+
                      |                                 |
                      v                                 v
       +------------------------------+  +------------------------------+
       |    Iterate catalog pages     |  |    Iterate product pages     |
       +------------------------------+  +------------------------------+
```

## Future test scenarios

- Single 30-inner-width box centered in 80 columns.
- Multiple boxes on one row centered as a group.
- Multi-line labels centered inside boxes.
- Branch connectors centered between parent and child boxes.
- Existing Bodul-style flowcharts remain stable after formatting.
- Non-graph fenced `text` blocks remain byte-identical.
- Non-`text` code fences and unfenced markdown remain byte-identical.
- Running the tool on already aligned ASCII graphs is idempotent.

## Assumptions

- `align-graph` only operates on fenced `text` code blocks.
- Go and Rust implementations will be kept byte-for-byte compatible through
  shared fixtures.
- The default target line span is columns `0..80`; there is no width flag in the
  initial plan.
