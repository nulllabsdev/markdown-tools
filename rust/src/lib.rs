//! Aligns the columns of GitHub-style pipe tables in markdown so that they read
//! cleanly as raw text. See `README.md` for the full specification.
//!
//! This is the Rust counterpart of the Go `mdtable` package; both are verified
//! against the same fixtures in `testdata/` to guarantee identical output.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use unicode_width::UnicodeWidthStr;

/// The smallest column width the formatter will emit, so that every separator
/// form (e.g. `:-:`) stays valid even for one-character columns.
const MIN_COL_WIDTH: usize = 3;

/// How a column's cells are positioned. `Default` and `Left` both left-align
/// content; they differ only in how the separator row is rendered (plain dashes
/// vs. a leading colon).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Default,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy)]
struct LogicalLine<'a> {
    text: &'a str,
    eol: &'a str,
}

/// Display width of `s`, measured with ambiguous-width characters treated as
/// narrow (matching the Go implementation's `EastAsianWidth: false`).
fn disp_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Formats every recognised pipe table in `input` and returns the result. Pure
/// and side-effect free. The original newline style (LF vs CRLF) and
/// trailing-newline state are preserved.
pub fn format_str(input: &str) -> String {
    let lines = split_lines(input);

    let mut out: Vec<(String, &str)> = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    let mut fence_marker = 0u8;
    let mut fence_len = 0usize;

    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].text.trim();

        // Code-fence boundaries pass through and toggle fence state.
        if !in_fence {
            if let Some((marker, n)) = opening_fence_token(trimmed) {
                in_fence = true;
                fence_marker = marker;
                fence_len = n;
                out.push((lines[i].text.to_string(), lines[i].eol));
                i += 1;
                continue;
            }
        } else if is_closing_fence(trimmed, fence_marker, fence_len) {
            in_fence = false;
            out.push((lines[i].text.to_string(), lines[i].eol));
            i += 1;
            continue;
        }
        if in_fence {
            out.push((lines[i].text.to_string(), lines[i].eol));
            i += 1;
            continue;
        }

        // A table is a pipe row immediately followed by a valid separator row.
        if i + 1 < lines.len() && is_pipe_row(lines[i].text) && is_separator_row(lines[i + 1].text)
        {
            let mut j = i + 2;
            while j < lines.len() && is_pipe_row(lines[j].text) {
                j += 1;
            }
            let formatted = format_table(&line_texts(&lines[i..j]));
            out.extend(
                formatted
                    .into_iter()
                    .enumerate()
                    .map(|(n, text)| (text, lines[i + n].eol)),
            );
            i = j;
            continue;
        }

        out.push((lines[i].text.to_string(), lines[i].eol));
        i += 1;
    }

    let mut result = String::new();
    for (text, eol) in out {
        result.push_str(&text);
        result.push_str(eol);
    }
    result
}

fn split_lines(input: &str) -> Vec<LogicalLine<'_>> {
    if input.is_empty() {
        return vec![LogicalLine { text: "", eol: "" }];
    }

    let mut lines =
        Vec::with_capacity(input.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1);
    let mut start = 0;
    for (i, b) in input.bytes().enumerate() {
        if b != b'\n' {
            continue;
        }
        let (end, eol) = if i > start && input.as_bytes()[i - 1] == b'\r' {
            (i - 1, "\r\n")
        } else {
            (i, "\n")
        };
        lines.push(LogicalLine {
            text: &input[start..end],
            eol,
        });
        start = i + 1;
    }
    if start < input.len() {
        lines.push(LogicalLine {
            text: &input[start..],
            eol: "",
        });
    }
    lines
}

fn line_texts<'a>(lines: &[LogicalLine<'a>]) -> Vec<&'a str> {
    lines.iter().map(|line| line.text).collect()
}

/// Walks `root` recursively and formats every `*.md` file in place, rewriting
/// only files whose content actually changes. Returns the paths of the files it
/// rewrote, in walk order.
pub fn format_directory(root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    let mut changed = Vec::new();
    visit_path(root.as_ref(), &mut changed)?;
    Ok(changed)
}

fn visit_path(path: &Path, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        return visit_dir(path, changed);
    }
    if metadata.is_file() && is_markdown(path) {
        format_path(path, changed)?;
    }
    Ok(())
}

fn visit_dir(dir: &Path, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            visit_dir(&path, changed)?;
        } else if is_markdown(&path) {
            format_path(&path, changed)?;
        }
    }
    Ok(())
}

fn format_path(path: &Path, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let data = fs::read_to_string(path)?;
    let formatted = format_str(&data);
    if formatted != data {
        // Writing to the existing file keeps its permissions.
        fs::write(path, formatted)?;
        changed.push(path.to_path_buf());
    }
    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

/// Reports whether a trimmed line opens a fenced code block, returning the
/// fence byte and run length. Opening fences may include an info string after
/// the marker run.
fn opening_fence_token(trimmed: &str) -> Option<(u8, usize)> {
    let bytes = trimmed.as_bytes();
    if bytes.len() < 3 {
        return None;
    }
    let c = bytes[0];
    if c != b'`' && c != b'~' {
        return None;
    }
    let n = bytes.iter().take_while(|&&b| b == c).count();
    if n < 3 {
        return None;
    }
    Some((c, n))
}

/// Reports whether a trimmed line closes the current fenced code block. Closing
/// fences must contain only the opening fence byte and must be at least as long
/// as the opening fence.
fn is_closing_fence(trimmed: &str, marker: u8, min_len: usize) -> bool {
    trimmed.len() >= min_len && trimmed.bytes().all(|b| b == marker)
}

/// Reports whether a line's trimmed text starts and ends with `|`.
fn is_pipe_row(line: &str) -> bool {
    let t = line.trim().as_bytes();
    t.len() >= 2 && t[0] == b'|' && t[t.len() - 1] == b'|'
}

/// Reports whether a line is a pipe row whose every cell is a valid alignment
/// separator (optional colons around one or more dashes).
fn is_separator_row(line: &str) -> bool {
    if !is_pipe_row(line) {
        return false;
    }
    let cells = split_cells(line);
    !cells.is_empty() && cells.iter().all(|c| is_separator_cell(c))
}

fn is_separator_cell(c: &str) -> bool {
    let b = c.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i] == b':' {
        i += 1;
    }
    let mut dashes = 0;
    while i < b.len() && b[i] == b'-' {
        i += 1;
        dashes += 1;
    }
    if dashes == 0 {
        return false;
    }
    if i < b.len() && b[i] == b':' {
        i += 1;
    }
    i == b.len()
}

/// Strips the leading and trailing pipe of a pipe row and splits the remainder
/// on unescaped `|`, trimming surrounding spaces from each cell. An escaped pipe
/// (`\|`) is preserved as literal content. Slicing happens only at ASCII `|`
/// boundaries, so every cell is valid UTF-8.
fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let inner = &t[1..t.len() - 1];
    let bytes = inner.as_bytes();

    let mut cells = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'|' => {
                cells.push(inner[start..i].trim().to_string());
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    cells.push(inner[start..].trim().to_string());
    cells
}

/// Formats a table block: header (`block[0]`), separator (`block[1]`), and zero
/// or more body rows.
fn format_table(block: &[&str]) -> Vec<String> {
    let header = split_cells(block[0]);
    let sep = split_cells(block[1]);
    let body: Vec<Vec<String>> = block[2..].iter().map(|row| split_cells(row)).collect();

    let num_cols = header
        .len()
        .max(sep.len())
        .max(body.iter().map(Vec::len).max().unwrap_or(0));

    let aligns: Vec<Align> = (0..num_cols)
        .map(|c| sep.get(c).map_or(Align::Default, |s| parse_align(s)))
        .collect();

    // Column widths come from header and body cells only (not the separator),
    // floored at the minimum width.
    let mut widths = vec![MIN_COL_WIDTH; num_cols];
    let mut consider = |cells: &[String]| {
        for (c, cell) in cells.iter().enumerate() {
            let w = disp_width(cell);
            if w > widths[c] {
                widths[c] = w;
            }
        }
    };
    consider(&header);
    for r in &body {
        consider(r);
    }

    let mut out = Vec::with_capacity(block.len());
    out.push(render_row(&header, &widths, &aligns));
    out.push(render_separator(&widths, &aligns));
    for r in &body {
        out.push(render_row(r, &widths, &aligns));
    }
    out
}

fn parse_align(sep_cell: &str) -> Align {
    let left = sep_cell.starts_with(':');
    let right = sep_cell.ends_with(':');
    match (left, right) {
        (true, true) => Align::Center,
        (false, true) => Align::Right,
        (true, false) => Align::Left,
        (false, false) => Align::Default,
    }
}

fn render_row(cells: &[String], widths: &[usize], aligns: &[Align]) -> String {
    let fields: Vec<String> = (0..widths.len())
        .map(|c| {
            let content = cells.get(c).map_or("", String::as_str);
            pad(content, widths[c], aligns[c])
        })
        .collect();
    format!("| {} |", fields.join(" | "))
}

/// Positions `content` within `width` display columns. Spaces are width 1, so
/// the space count equals the width deficit. Centring puts any odd extra space
/// on the right.
fn pad(content: &str, width: usize, align: Align) -> String {
    let total = width.saturating_sub(disp_width(content));
    match align {
        Align::Right => format!("{}{}", " ".repeat(total), content),
        Align::Center => {
            let left = total / 2;
            format!(
                "{}{}{}",
                " ".repeat(left),
                content,
                " ".repeat(total - left)
            )
        }
        Align::Default | Align::Left => format!("{}{}", content, " ".repeat(total)),
    }
}

fn render_separator(widths: &[usize], aligns: &[Align]) -> String {
    let fields: Vec<String> = widths
        .iter()
        .zip(aligns)
        .map(|(&w, &a)| sep_field(w, a))
        .collect();
    format!("| {} |", fields.join(" | "))
}

fn sep_field(width: usize, align: Align) -> String {
    match align {
        Align::Left => format!(":{}", "-".repeat(width - 1)),
        Align::Right => format!("{}:", "-".repeat(width - 1)),
        Align::Center => format!(":{}:", "-".repeat(width - 2)),
        Align::Default => "-".repeat(width),
    }
}
