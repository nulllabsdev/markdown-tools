//! Reflows markdown prose paragraphs to a display width. The Rust counterpart of
//! the Go `mdtable` wrapper; both are verified against the shared `testdata/wrap`
//! fixtures. Reuses the aligner's line, fence, and table primitives from the
//! crate root.

use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::common::{
    disp_width, indented_too_far, is_closing_fence, is_markdown, is_pipe_row, opening_fence_token,
    split_lines, LogicalLine,
};

/// Reflows prose paragraphs in `input` to `width` display columns and returns the
/// result. Pure and side-effect free: fenced code blocks, tables, headings,
/// lists, blockquotes, thematic breaks, HTML blocks, indented code, and YAML
/// front matter pass through untouched, and the original newline style (LF vs
/// CRLF) and trailing-newline state are preserved. A `width` of 0 disables
/// wrapping (each paragraph collapses to a single line).
pub fn wrap_str(input: &str, width: usize) -> String {
    let lines = split_lines(input);

    let mut out: Vec<(Cow<str>, &str)> = Vec::with_capacity(lines.len());
    let mut para: Vec<LogicalLine> = Vec::new();

    let mut in_fence = false;
    let mut fence_marker = 0u8;
    let mut fence_len = 0usize;
    let mut in_front_matter = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.text.trim();

        // YAML front matter: a leading `---` on the very first line opens a block
        // that passes through verbatim until its closing `---`/`...`.
        if i == 0 && trimmed == "---" {
            in_front_matter = true;
            out.push((Cow::Borrowed(line.text), line.eol));
            continue;
        }
        if in_front_matter {
            out.push((Cow::Borrowed(line.text), line.eol));
            if trimmed == "---" || trimmed == "..." {
                in_front_matter = false;
            }
            continue;
        }

        // Code fences pass through and toggle fence state (reusing the aligner's
        // CommonMark-aware detection).
        if !indented_too_far(line.text) {
            if !in_fence {
                if let Some((marker, n)) = opening_fence_token(trimmed) {
                    flush(&mut out, &mut para, width);
                    in_fence = true;
                    fence_marker = marker;
                    fence_len = n;
                    out.push((Cow::Borrowed(line.text), line.eol));
                    continue;
                }
            } else if is_closing_fence(trimmed, fence_marker, fence_len) {
                in_fence = false;
                out.push((Cow::Borrowed(line.text), line.eol));
                continue;
            }
        }
        if in_fence {
            out.push((Cow::Borrowed(line.text), line.eol));
            continue;
        }

        // A pending paragraph immediately followed by a setext underline is a
        // heading; emit both verbatim so the underline keeps matching. (Checked
        // before prose classification because "===" is not otherwise a block.)
        if !para.is_empty() && is_setext_underline(line.text) {
            for l in &para {
                out.push((Cow::Borrowed(l.text), l.eol));
            }
            para.clear();
            out.push((Cow::Borrowed(line.text), line.eol));
            continue;
        }

        if is_prose(line.text) {
            para.push(*line);
            continue;
        }

        // Any non-prose line ends the current paragraph and passes through.
        flush(&mut out, &mut para, width);
        out.push((Cow::Borrowed(line.text), line.eol));
    }
    flush(&mut out, &mut para, width);

    let mut result = String::new();
    for (text, eol) in out {
        result.push_str(&text);
        result.push_str(eol);
    }
    result
}

/// Walks `root` recursively and wraps every `*.md` file in place to `width`
/// columns, rewriting only files whose content actually changes. Returns the
/// paths of the files it rewrote, in walk order.
pub fn wrap_directory(root: impl AsRef<Path>, width: usize) -> io::Result<Vec<PathBuf>> {
    let mut changed = Vec::new();
    visit_path(root.as_ref(), width, &mut changed)?;
    Ok(changed)
}

fn visit_path(path: &Path, width: usize, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        return visit_dir(path, width, changed);
    }
    if metadata.is_file() && is_markdown(path) {
        wrap_path(path, width, changed)?;
    }
    Ok(())
}

fn visit_dir(dir: &Path, width: usize, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            visit_dir(&path, width, changed)?;
        } else if is_markdown(&path) {
            wrap_path(&path, width, changed)?;
        }
    }
    Ok(())
}

fn wrap_path(path: &Path, width: usize, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let data = fs::read_to_string(path)?;
    let wrapped = wrap_str(&data, width);
    if wrapped != data {
        // Writing to the existing file keeps its permissions.
        fs::write(path, wrapped)?;
        changed.push(path.to_path_buf());
    }
    Ok(())
}

/// Wraps the pending paragraph (if any) into `out` and clears it.
fn flush<'a>(
    out: &mut Vec<(Cow<'a, str>, &'a str)>,
    para: &mut Vec<LogicalLine<'a>>,
    width: usize,
) {
    if para.is_empty() {
        return;
    }
    for (text, eol) in wrap_paragraph(para, width) {
        out.push((Cow::Owned(text), eol));
    }
    para.clear();
}

/// Greedily packs the words of a prose paragraph into lines no wider than `width`
/// display columns, never splitting a single word. Newly created lines reuse the
/// paragraph's first source eol; the final line keeps the last source eol so
/// trailing-newline and CRLF state survive.
fn wrap_paragraph<'a>(para: &[LogicalLine<'a>], width: usize) -> Vec<(String, &'a str)> {
    let mut words: Vec<&str> = Vec::new();
    for line in para {
        words.extend(line.text.split_whitespace());
    }
    if words.is_empty() {
        return para.iter().map(|l| (l.text.to_string(), l.eol)).collect();
    }

    let first_eol = para[0].eol;
    let last_eol = para[para.len() - 1].eol;

    let mut texts: Vec<String> = Vec::new();
    if width == 0 {
        texts.push(words.join(" "));
    } else {
        let mut cur = String::from(words[0]);
        let mut cur_w = disp_width(words[0]);
        for w in &words[1..] {
            let ww = disp_width(w);
            if cur_w + 1 + ww <= width {
                cur.push(' ');
                cur.push_str(w);
                cur_w += 1 + ww;
            } else {
                texts.push(std::mem::take(&mut cur));
                cur = String::from(*w);
                cur_w = ww;
            }
        }
        texts.push(cur);
    }

    let last = texts.len() - 1;
    texts
        .into_iter()
        .enumerate()
        .map(|(i, t)| (t, if i == last { last_eol } else { first_eol }))
        .collect()
}

/// Reports whether a line is ordinary paragraph text — it has content and is none
/// of the block constructs that must pass through untouched. Fenced code,
/// indented code, and front matter are handled by the caller before this.
fn is_prose(line: &str) -> bool {
    if line.trim().is_empty() || indented_too_far(line) {
        return false;
    }
    !(is_atx_heading(line)
        || is_blockquote(line)
        || is_list_item(line)
        || is_thematic_break(line)
        || is_html_block_start(line)
        || is_pipe_row(line))
}

/// Count of leading ASCII spaces, capped at the 4 that matter for CommonMark.
fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').take(4).count()
}

/// ATX heading: 1-6 `#` then a space or end of line, indented at most 3 columns.
fn is_atx_heading(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    let b = &line.as_bytes()[leading_spaces(line)..];
    let hashes = b.iter().take_while(|&&c| c == b'#').count();
    if !(1..=6).contains(&hashes) {
        return false;
    }
    match b.get(hashes) {
        None => true,
        Some(&c) => c == b' ' || c == b'\t',
    }
}

/// Block quote: a `>` at most three columns in.
fn is_blockquote(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    line.as_bytes().get(leading_spaces(line)) == Some(&b'>')
}

/// Bullet or ordered list item marker.
fn is_list_item(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    let b = &line.as_bytes()[leading_spaces(line)..];
    if b.is_empty() {
        return false;
    }
    if matches!(b[0], b'-' | b'*' | b'+') {
        return b.len() == 1 || b[1] == b' ' || b[1] == b'\t';
    }
    let digits = b.iter().take(9).take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits >= b.len() {
        return false;
    }
    if b[digits] != b'.' && b[digits] != b')' {
        return false;
    }
    let rest = digits + 1;
    rest == b.len() || b[rest] == b' ' || b[rest] == b'\t'
}

/// Thematic break: three or more of a single `-`, `*`, or `_` with only spaces
/// between them.
fn is_thematic_break(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    let s = line.trim();
    if s.len() < 3 {
        return false;
    }
    let marker = s.as_bytes()[0];
    if !matches!(marker, b'-' | b'*' | b'_') {
        return false;
    }
    let mut count = 0;
    for &b in s.as_bytes() {
        if b == marker {
            count += 1;
        } else if b != b' ' {
            return false;
        }
    }
    count >= 3
}

/// Setext heading underline: a run of only `=` or only `-`, indented ≤3 columns.
fn is_setext_underline(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    let s = line[leading_spaces(line)..].trim_end_matches([' ', '\t']);
    if s.is_empty() {
        return false;
    }
    let marker = s.as_bytes()[0];
    if marker != b'=' && marker != b'-' {
        return false;
    }
    s.bytes().all(|b| b == marker)
}

/// HTML block start: a `<` at most three columns in.
fn is_html_block_start(line: &str) -> bool {
    if indented_too_far(line) {
        return false;
    }
    line.as_bytes().get(leading_spaces(line)) == Some(&b'<')
}
