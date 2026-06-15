# Design: Rust markdown table column aligner

## Context

`README.md` specifies a tool that aligns the columns of GitHub-style pipe tables
in markdown so they read cleanly as raw text. This document records the design of
the **Rust** implementation (crate `markdown_tools`, under `/rust`). It is a
faithful port of the Go `mdtable` package (`go/align.go`) — the algorithm is
identical, only the language idioms differ — and is verified against the same
shared fixtures in `testdata/`, guaranteeing byte-for-byte parity with Go.

See `docs/go-table-aligner.md` for the sibling Go design.

## Behaviour (the contract)

- Recognise only **fully-piped** tables: a header row + a valid separator row,
  every row's trimmed text starting and ending with `|`.
- Column width = **display width** (CJK / most emoji = 2, combining marks = 0),
  with a **minimum column width of 3**.
- Respect separator markers (`:--` left, `:-:` centre, `--:` right); no marker =
  default left. Content is aligned to its marker, **including the header**.
- Separator cells filled with dashes to the column width, colons kept at the
  marked end(s). Default = plain dashes; explicit left keeps its leading colon.
- Centre rounding: an odd extra space goes on the **right**.
- `\|` is literal cell content (not a column break) and counts as **width 2**.
- Ragged rows padded with empty cells; tables inside fenced code blocks
  (```` ``` ````/`~~~`) left untouched.
- Idempotent; original line endings (LF vs CRLF) and trailing-newline preserved.

## Layout

```
/rust
  Cargo.toml          name = "markdown_tools", edition = "2021"
  Cargo.lock
  src/lib.rs          public API + core formatter
  tests/fixtures.rs   integration tests over ../testdata
```

Dependency: `unicode-width` (0.2) for display width. Tests resolve fixtures via
`env!("CARGO_MANIFEST_DIR")/../testdata`. Directory walking uses `std::fs`
recursion — no `walkdir` dependency.

## Public API

- `pub fn format_str(input: &str) -> String` — pure, in-memory formatter; the
  primary test target.
- `pub fn format_directory(root: impl AsRef<Path>) -> std::io::Result<()>` —
  recurse with `std::fs::read_dir`, formatting every `*.md` file in place (read →
  `format_str` → write back only when the content changes; `fs::write` on an
  existing file keeps its permissions).

## Implementation notes (`src/lib.rs`) — mirrors `go/align.go`

**Deterministic width.** Uses `unicode_width::UnicodeWidthStr::width` (ambiguous
= narrow), matching Go's `runewidth.Condition{EastAsianWidth: false}`. CJK and
`🍎` are width 2, combining marks width 0. Because `\|` is kept literal, `.width()`
counts the backslash + pipe as 2 with no special case. Parity risk is contained:
any disagreement between `unicode-width` and `go-runewidth` would fail a fixture
loudly, and the fixtures use only widely-agreed characters.

**Line handling.** Newline is `\r\n` if present in the input, else `\n`. The
trailing-newline state is recorded, CRLF normalised to LF for processing via
`replace`/`split('\n')`, and both restored on output.

**Main loop.** Walks logical lines tracking code-fence state. `fence_token`
detects a boundary (≥3 leading `` ` `` or `~` after trimming); inside a fence,
lines pass through verbatim. Outside a fence, a table starts where line *i* is a
pipe row and line *i+1* is a valid separator row; the header, separator, and
following pipe rows are collected, formatted, and emitted.

**Cell parsing (`split_cells`).** Strips one leading + one trailing `|` (ASCII,
safe slice), scans the inner string's **bytes** for unescaped `|` delimiters
(skipping the pair after `\`), slices cells at those ASCII boundaries (valid
UTF-8), and trims each. Byte scanning is safe because `|` and `\` never appear
inside a multibyte UTF-8 sequence.

**Column model & rendering.** `num_cols = max(header, separator, body)`; shorter
rows padded with empty cells. Alignment per column derives from the separator
cell — `enum Align { Default, Left, Center, Right }`, where Default and Left
differ only in separator rendering. Width = `max(3, max .width() over
header+body cells)`. Each row becomes `format!("| {} |", fields.join(" | "))`,
each cell padded with `" ".repeat(n)` by alignment (Default/Left right-pad, Right
left-pad, Center with the extra space on the right). The separator renders per
column as `-`×w, `:`+`-`×(w-1), `-`×(w-1)+`:`, or `:`+`-`×(w-2)+`:`; the minimum
width of 3 keeps every form valid.

## Verification

From `/rust`:

```
cargo build && cargo test && cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

`tests/fixtures.rs` runs three checks:

1. **Fixtures** — for each `../testdata/*.input`, assert
   `format_str(input) == matching .output`.
2. **Idempotency** — assert `format_str(output) == output` for every `.output`.
3. **format_directory** — copy fixtures (renamed `*.md`) into a temp dir,
   including a nested subdirectory and a non-`.md` file; run `format_directory`;
   assert the `*.md` files now equal their `.output` and the non-`.md` file is
   untouched.

All eight fixtures pass, matching the Go results exactly: three-alignments,
no-markers, cjk, emoji, empty-ragged, centered-single-char, escaped-pipe,
code-fence-skip.

## Out of scope

CLI binary (the spec is library-only), inline-code-span pipe parsing beyond `\|`,
and tables without leading/trailing pipes.
