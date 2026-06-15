# markdown-tools
Set of tools to help with managing markdown formats

## Align columns in markdown tables

Aligns the columns of GitHub-style pipe tables so they are readable in raw
markdown. Each column is padded to the minimal width needed to fit its longest
cell, with one space of padding around every cell and separator.

### Behaviour

- **Form:** a library exposing a pure core plus a directory walker (see API
  below), and a thin CLI for each language (`go-align`, `rust-align`).
- **Scope:** processes a directory and all its subfolders, limited to `*.md`
  files, editing each file **in place**. Original line endings (LF vs CRLF) and
  trailing-newline state are preserved.
- **Reporting:** when formatting in place, the CLIs print the full path of every
  file they changed, one per line; files that are already aligned print nothing.
- **Recognised tables:** only fully-piped tables — every row, the header, and
  the separator line must have a leading and trailing `|`. A valid table needs
  both a header row and a separator line.
- **Width = display width:** column widths are measured in terminal columns, not
  bytes or code points. CJK characters and most emoji count as 2, combining
  marks as 0. (Rust: `unicode-width`; Go: `github.com/mattn/go-runewidth`.)
- **Alignment markers are respected:** `:---` left, `:--:` centre, `---:` right;
  a column with no marker defaults to **left**. Cell content is aligned to match
  its column's marker.
- **Separator rendering:** the separator cell is filled with dashes to the
  column width, keeping colons at the marked end(s).
- **Centre rounding:** when centring leaves an odd extra space, it goes on the
  **right**.
- **Minimum column width:** 3, so every separator stays GFM-valid even for
  one-character columns.
- **Escaped pipes:** `\|` inside a cell is treated as literal content (not a
  column break) and counted as **width 2** (both glyphs are visible in raw
  markdown).
- **Ragged rows:** a row with fewer cells than the header is padded with empty
  cells to match.
- **Code fences:** pipe tables inside fenced code blocks (```` ``` ````) are left
  untouched.
- **Idempotent:** running the tool on already-aligned output produces
  byte-identical output.

### Examples

**The three alignments**
```
| Name  | Age |        City |
| :---- | :-: | ----------: |
| Alice | 30  |         NYC |
| Bob   |  5  | Los Angeles |
```

**No markers (default left), narrow columns hit the minimum width of 3**
```
| a   | bb  | ccc |
| --- | --- | --- |
| 1   | 2   | 3   |
```

**CJK (display width).** `你好` / `東京` occupy 4 columns and line up with
`Name`/`City`; `NYC` (3) gets one trailing space.
```
| Name | City |
| ---- | ---- |
| 你好 | NYC  |
| Bob  | 東京 |
```

**Emoji.** `🍎` is width 2, padded out to the column's 4.
```
| Item | Qty |
| ---- | --- |
| 🍎   | 3   |
| Pear | 10  |
```

**Empty cell + ragged-row padding.** The third row supplies only 2 cells and is
padded to the header's 3.
```
| A   | B   | C   |
| --- | --- | --- |
| x   |     |     |
```

**Single-char centred column.** Width 1 is bumped to the minimum of 3; header
and body are both centred.
```
|  x  |
| :-: |
|  a  |
```

**Escaped pipe.** `\|` stays literal and counts as width 2.
```
| Expr   | Val |
| ------ | --- |
| a \| b | 1   |
```

**Code fence is skipped.** The table inside the fence is left exactly as
written; only the one below it is formatted.
````
```
| not | touched |
|-|-|
```

| Yes | Formatted |
| --- | --------- |
````

### Requirements

- columns should be aligned by using minimal width needed for it (meaning we
  will align to the longest text)
- when calculating, we need to consider one space around any text or separators
- tool needs to be written in both Rust and Golang

## Wrap markdown prose

Reflows prose paragraphs so they fit within a display width (default 80 columns,
`-n N` to override), making raw markdown comfortable to read and review. Wrapping
is greedy and never splits a word, so long tokens such as URLs keep their own
line.

### Behaviour

- **Prose only:** ordinary paragraph text is reflowed; fenced code blocks,
  indented code, tables, ATX and setext headings, lists, blockquotes, thematic
  breaks, HTML blocks, and YAML front matter pass through untouched.
- **Width = display width:** measured in terminal columns (CJK counts as 2), the
  same metric the aligner uses.
- **Paragraph boundaries, line endings (LF vs CRLF), and trailing-newline state
  are preserved.** Wrapping already-wrapped prose is idempotent.
- **Reporting & scope:** like the aligner, directory inputs are processed
  recursively for `*.md` only, files are rewritten only when content changes, and
  each changed path is printed one per line.

Known limitations: prose is normalized to column 0 and trailing whitespace is
trimmed; two-space "hard breaks" inside a paragraph are not preserved.

```
$ bin/go-wrap README.md
README.md

$ bin/rust-wrap -n 100 docs/
docs/guide.md
```

## Align ASCII graphs

Re-renders ASCII flowcharts inside fenced ` ```text ` code blocks to a canonical,
centered form so they read cleanly in raw markdown.

### Behaviour

- **Boxes** are normalized to a 30-character inner width with their label
  centered (odd extra space on the right).
- **Box-rows** (one or more boxes side by side, joined by two spaces) are centered
  as a group within 80 columns.
- **Structural connector lines** (made only of `|`, `v`, `+`, `-`) are re-centered
  in 80 columns so vertical and 2-way branch connectors line up beneath the boxes;
  **text annotations** on connectors (e.g. `for each item`) keep their exact
  position.
- **Scope:** only ` ```text ` blocks that contain at least one box are touched.
  Non-graph ` ```text ` blocks, non-`text` fences, and all unfenced markdown pass
  through byte-for-byte. Line endings and trailing-newline state are preserved,
  and already-aligned graphs are idempotent.

Known limitation: a box whose label exceeds 30 display columns leaves its whole
graph block untouched (labels are centered, never re-wrapped).

```
$ bin/go-align-graph README.md
README.md
```

## CLI

Build the binaries with `make` (output lands in `bin/`):

```
make                  # build all six binaries
make go-align         # aligner (Go)        make rust-align       # aligner (Rust)
make go-wrap          # wrapper (Go)        make rust-wrap        # wrapper (Rust)
make go-align-graph   # graph (Go)          make rust-align-graph # graph (Rust)
```

There are three tools — `*-align` (table aligner), `*-wrap` (prose wrapper), and
`*-align-graph` (ASCII-graph aligner) — each in a Go and a Rust build. They share
the same interface:

- **No arguments** — read markdown from stdin, write the result to stdout.
- **One or more paths** — process each in place. A directory is walked
  recursively for `*.md` files; a file is rewritten only if its content changes.
  The full path of every file that changed is printed to stdout, one per line.
- The wrapper additionally accepts `-n N` to set the wrap width (default 80).

```
$ bin/go-align docs/
docs/guide.md
docs/api/reference.md

$ cat table.md | bin/rust-align        # stdin → stdout, nothing else printed

$ bin/go-wrap -n 100 docs/             # wrap prose to 100 columns in place
docs/guide.md
```

## Project layout

```
/rust       Rust crate `markdown_tools` (bins rust-align, rust-wrap, rust-align-graph)
/go         Go package `mdtable` (cmds go-align, go-wrap, go-align-graph, Go 1.22+)
/testdata        Shared aligner fixtures: `name.input` / `name.output`
/testdata/wrap   Shared wrapper fixtures (run at width 80)
/testdata/graph  Shared ASCII-graph fixtures
/Makefile   Builds all six CLIs into /bin
```

Both implementations are verified against the same `*.input` / `*.output`
fixtures so their output is provably identical. Each test reads an `.input`
file, processes it, and asserts the result equals the matching `.output` file
byte-for-byte.

### Public API

Each language exposes the same pair of entry points for every tool:

- `format_str` / `wrap_str` / `align_graph_str` — pure, in-memory string → string;
  the primary unit-test targets (`wrap_str` also takes a width).
- `format_directory` / `wrap_directory` / `align_graph_directory` — recursively
  find `*.md` under a path and rewrite each in place, returning the paths of the
  files they changed (Go: `[]string`; Rust: `Vec<PathBuf>`) so callers can report
  them.

### Dependencies

- Rust: `unicode-width`
- Go: `github.com/mattn/go-runewidth`
