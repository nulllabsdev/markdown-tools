# markdown-tools
Set of tools to help with managing markdown formats

## Align columns in markdown tables

Aligns the columns of GitHub-style pipe tables so they are readable in raw
markdown. Each column is padded to the minimal width needed to fit its longest
cell, with one space of padding around every cell and separator.

### Behaviour

- **Form:** a library exposing a pure core plus a directory walker (see API
  below). No CLI is required.
- **Scope:** processes a directory and all its subfolders, limited to `*.md`
  files, editing each file **in place**. Original line endings (LF vs CRLF) and
  trailing-newline state are preserved.
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
| Name  | Age | City        |
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

## Project layout

```
/rust       Rust implementation (lib crate `markdown_tools`, edition 2021)
/go         Go implementation (package `mdtable`, Go 1.22+)
/testdata   Shared fixtures: `name.in.md` / `name.out.md` pairs run by both
```

Both implementations are verified against the same `*.in.md` / `*.out.md`
fixtures so their output is provably identical.

### Public API

Each language exposes the same two entry points:

- `format_str` — pure, in-memory string → string; the primary unit-test target.
- `format_directory` — recursively finds `*.md` under a path and rewrites each
  in place.

### Dependencies

- Rust: `unicode-width`
- Go: `github.com/mattn/go-runewidth`
