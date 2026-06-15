# Design: Go markdown table column aligner

## Context

`README.md` specifies a tool that aligns the columns of GitHub-style pipe tables
in markdown so they read cleanly as raw text. This document records the design
of the **Go** implementation (package `mdtable`, under `/go`), which is verified
against the shared fixture suite in `testdata/`. The Rust implementation uses
the same fixtures, guaranteeing byte-for-byte parity between the two.

## Behaviour (the contract)

- Recognise only **fully-piped** tables: a header row + a valid separator row,
  every row's trimmed text starting and ending with `|`.
- Column width = **display width** (CJK / most emoji = 2, combining marks = 0),
  with a **minimum column width of 3**.
- Respect separator alignment markers (`:--` left, `:-:` centre, `--:` right);
  no marker = default left. Content is aligned to match its marker, **including
  the header row**.
- Separator cells are filled with dashes to the column width, colons kept at the
  marked end(s). Default (no marker) renders as plain dashes; explicit left keeps
  its leading colon.
- Centre rounding: an odd extra space goes on the **right**.
- `\|` is literal cell content (not a column break) and counts as **width 2**.
- Ragged rows are padded with empty cells; tables inside fenced code blocks
  (```` ``` ````/`~~~`) are left untouched.
- Idempotent; original line endings (LF vs CRLF) and trailing-newline state are
  preserved.

## Layout

```
/go
  go.mod          module github.com/nulllabsdev/markdown-tools/go  (go 1.22)
  go.sum
  align.go        package mdtable — public API + core formatter
  align_test.go   fixture-driven + idempotency + FormatDirectory tests
  cmd/go-align    stdin/stdout and in-place CLI wrapper
```

Dependency: `github.com/mattn/go-runewidth` for display width. Tests read the
shared fixtures via the relative path `../testdata`.

## Public API

- `FormatString(s string) string` — pure, in-memory formatter; the primary test
  target.
- `FormatDirectory(root string) ([]string, error)` — `filepath.WalkDir` over
  `root`, formatting every `*.md` file in place (read → `FormatString` → write
  back only when the content changes, preserving file mode) and returning the
  changed paths in walk order.

## Implementation notes (`align.go`)

**Line handling.** Input is split into logical lines while retaining each line's
original terminator (`\n`, `\r\n`, or none for the final unterminated line).
Formatted output reuses the original terminator for each emitted line, so mixed
line endings and trailing-newline state are preserved exactly.

**Deterministic width.** A single `&runewidth.Condition{EastAsianWidth: false}`
is used via `StringWidth`, so ambiguous-width runes are narrow regardless of the
`LANG` environment while CJK and emoji stay width 2. Because `\|` is kept literal
in cell content, `StringWidth` counts the backslash + pipe as 2 with no special
case.

**Main loop.** Walks logical lines tracking code-fence state. Opening fences are
detected from a leading run of at least three backticks or tildes and may include
an info string; closing fences must use the same marker, be at least as long as
the opener, and contain only marker characters after trimming. Lines inside a
fence pass through verbatim. Outside a fence, a table starts where line *i* is a
pipe row and line *i+1* is a valid separator row (every cell matches
`^:?-+:?$`). The header, separator, and consecutive body pipe rows are collected,
formatted, and emitted; everything else passes through unchanged.

**Cell parsing.** Strip one leading and one trailing `|`, split the remainder on
**unescaped** `|`, and trim spaces around each cell. Byte iteration is safe
because `|` and `\` are ASCII and never occur inside a multibyte UTF-8 sequence.

**Column model.** `numCols = max(header, separator, body)` cells; shorter rows
are padded with empty cells (covers ragged rows without dropping data). Per
column, alignment is derived from the separator cell — `Default`/`Left`/`Center`/
`Right`, where Default and Left differ only in separator rendering. Width =
`max(3, max display width over header + body cells)` (separator excluded).

**Rendering.** Each row becomes `| ` + cells joined by ` | ` + ` |`, each cell
padded to its column width by alignment (Default/Left right-pad, Right left-pad,
Center splits with the extra space on the right). The separator row renders per
column as `-`×w, `:`+`-`×(w-1), `-`×(w-1)+`:`, or `:`+`-`×(w-2)+`:`; the minimum
width of 3 keeps every form valid.

## Verification

From `/go`:

```
go mod tidy && go vet ./... && go build ./... && gofmt -l . && go test ./...
```

`align_test.go` runs three checks:

1. **Fixtures** — for each `../testdata/*.input`, assert
   `FormatString(input) == matching .output` byte-for-byte (one sub-test per
   fixture).
2. **Idempotency** — assert `FormatString(output) == output` for every
   `.output`.
3. **FormatDirectory** — copy fixtures (renamed `*.md`) into a temp dir,
   including a nested subdirectory and a non-`.md` file; run `FormatDirectory`;
   assert the `*.md` files now equal their `.output` and the non-`.md` file is
   untouched.

The fixture suite covers shared table behavior plus focused regression tests for
mixed line endings, idempotency, valid code-fence closing, and recursive
directory formatting.

## Out of scope

Inline-code-span pipe parsing beyond `\|`, and tables without leading/trailing
pipes.
