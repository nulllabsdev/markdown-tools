//! Re-renders ASCII flowcharts inside fenced ` ```text ` blocks to a canonical
//! form: 30-char inner boxes with centered labels, box-rows centered in 80
//! columns, and structural connector lines re-centered beneath the boxes. The
//! Rust counterpart of the Go `mdtable` align-graph; both are verified against
//! the shared `testdata/graph` fixtures. Reuses the aligner's primitives.

use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{
    disp_width, indented_too_far, is_closing_fence, is_markdown, opening_fence_token, split_lines,
    LogicalLine,
};

const INNER_WIDTH: usize = 30;
const SPAN: usize = 80;
const BOX_GAP: usize = 2;

/// Re-renders ASCII flowcharts inside fenced ` ```text ` blocks. Pure and
/// side-effect free: non-graph ` ```text ` blocks, non-`text` fences, and all
/// unfenced markdown pass through untouched, and the original newline style and
/// trailing-newline state are preserved.
pub fn align_graph_str(input: &str) -> String {
    let lines = split_lines(input);

    let mut out: Vec<(Cow<str>, &str)> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].text.trim();

        let token = if indented_too_far(lines[i].text) {
            None
        } else {
            opening_fence_token(trimmed)
        };
        let Some((marker, n)) = token else {
            out.push((Cow::Borrowed(lines[i].text), lines[i].eol));
            i += 1;
            continue;
        };

        // Opening fence: gather the block up to its closing fence.
        let info = trimmed[n..].trim();
        let mut j = i + 1;
        while j < lines.len() {
            let tj = lines[j].text.trim();
            if !indented_too_far(lines[j].text) && is_closing_fence(tj, marker, n) {
                break;
            }
            j += 1;
        }

        out.push((Cow::Borrowed(lines[i].text), lines[i].eol)); // opener
        let inner = &lines[i + 1..j];
        if info == "text" {
            for line in align_graph_block(inner) {
                out.push(line);
            }
        } else {
            for line in inner {
                out.push((Cow::Borrowed(line.text), line.eol));
            }
        }
        if j < lines.len() {
            out.push((Cow::Borrowed(lines[j].text), lines[j].eol)); // closer
            i = j + 1;
        } else {
            i = j;
        }
    }

    let mut result = String::new();
    for (text, eol) in out {
        result.push_str(&text);
        result.push_str(eol);
    }
    result
}

/// Walks `root` recursively and re-renders graphs in every `*.md` file in place,
/// rewriting only files whose content actually changes. Returns the paths of the
/// files it rewrote, in walk order.
pub fn align_graph_directory(root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
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
        align_path(path, changed)?;
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
            align_path(&path, changed)?;
        }
    }
    Ok(())
}

fn align_path(path: &Path, changed: &mut Vec<PathBuf>) -> io::Result<()> {
    let data = fs::read_to_string(path)?;
    let formatted = align_graph_str(&data);
    if formatted != data {
        fs::write(path, formatted)?;
        changed.push(path.to_path_buf());
    }
    Ok(())
}

/// Re-renders the inner lines of a ` ```text ` block if they form a graph
/// (contain at least one box-border row); otherwise the block is returned
/// unchanged. Also left untouched if any box label overflows the inner width.
fn align_graph_block<'a>(inner: &[LogicalLine<'a>]) -> Vec<(Cow<'a, str>, &'a str)> {
    let texts: Vec<&str> = inner.iter().map(|l| l.text).collect();
    if !texts.iter().any(|t| is_box_border_row(t)) {
        return inner
            .iter()
            .map(|l| (Cow::Borrowed(l.text), l.eol))
            .collect();
    }

    let eol = inner.first().map(|l| l.eol).unwrap_or("\n");
    match render_graph(&texts) {
        Some(rendered) => rendered
            .into_iter()
            .map(|text| (Cow::Owned(text), eol))
            .collect(),
        None => inner
            .iter()
            .map(|l| (Cow::Borrowed(l.text), l.eol))
            .collect(),
    }
}

/// Rebuilds every line of a graph block. Returns `None` if any box label exceeds
/// the inner width, signalling the caller to leave the block untouched.
fn render_graph(texts: &[&str]) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::with_capacity(texts.len());
    let mut i = 0;
    while i < texts.len() {
        if is_box_border_row(texts[i]) {
            let segs = box_segments(texts[i]);
            let mut j = i + 1;
            while j < texts.len() && !is_box_border_row(texts[j]) {
                j += 1;
            }
            let rows = render_box_row(&segs, &texts[i + 1..j])?;
            out.extend(rows);
            i = if j < texts.len() { j + 1 } else { j };
            continue;
        }
        out.push(transform_connector(texts[i]));
        i += 1;
    }
    Some(out)
}

/// Reports whether a line is one or more box borders side by side: each
/// 2-space-separated segment is `+`, one or more `-`, `+` (so a branch line like
/// `+----+----+`, which has an interior `+`, is NOT a border).
fn is_box_border_row(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    t.split("  ").all(|part| is_box_segment(part.trim()))
}

fn is_box_segment(part: &str) -> bool {
    let b = part.as_bytes();
    b.len() >= 3
        && b[0] == b'+'
        && b[b.len() - 1] == b'+'
        && b[1..b.len() - 1].iter().all(|&c| c == b'-')
}

/// Returns the [start, end] byte-column range of each box on a border line
/// (start at the leading `+`, end at the trailing `+`).
fn box_segments(line: &str) -> Vec<(usize, usize)> {
    let b = line.as_bytes();
    let mut segs = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'+' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < b.len() && b[i] == b'-' {
            i += 1;
        }
        if i < b.len() && b[i] == b'+' {
            segs.push((start, i));
            i += 1;
        }
    }
    segs
}

/// Normalises one row of boxes. `segs` gives the input column ranges (used only
/// to slice labels out of the content lines); the output geometry is canonical:
/// 30-inner boxes joined by two spaces and centered within 80 columns.
fn render_box_row(segs: &[(usize, usize)], content: &[&str]) -> Option<Vec<String>> {
    let n_boxes = segs.len();
    if n_boxes == 0 {
        return Some(Vec::new());
    }

    // labels[b] holds the label for each content line of box b.
    let mut labels: Vec<Vec<String>> = vec![Vec::new(); n_boxes];
    for (b, &seg) in segs.iter().enumerate() {
        for line in content {
            labels[b].push(cell_label(line, seg.0, seg.1));
        }
    }
    for box_labels in &labels {
        for label in box_labels {
            if disp_width(label) > INNER_WIDTH {
                return None;
            }
        }
    }

    let border = format!("+{}+", "-".repeat(INNER_WIDTH));
    let gap = " ".repeat(BOX_GAP);
    let border_row = vec![border.as_str(); n_boxes].join(&gap);

    let mut rows: Vec<String> = Vec::with_capacity(content.len() + 2);
    rows.push(border_row.clone());
    for r in 0..content.len() {
        let cells: Vec<String> = (0..n_boxes)
            .map(|b| {
                let label = labels[b].get(r).map(String::as_str).unwrap_or("");
                format!("|{}|", center_in_width(label, INNER_WIDTH))
            })
            .collect();
        rows.push(cells.join(&gap));
    }
    rows.push(border_row);

    Some(rows.into_iter().map(|row| center_in_span(&row)).collect())
}

/// Extracts and trims the label of a box cell occupying byte columns
/// [start, end] of a content line.
fn cell_label(line: &str, start: usize, end: usize) -> String {
    let b = line.as_bytes();
    if start >= b.len() {
        return String::new();
    }
    let end = end.min(b.len() - 1);
    let cell = &line[start..=end];
    cell.trim_start_matches('|')
        .trim_end_matches('|')
        .trim()
        .to_string()
}

/// Re-centers a structural connector line (only `|`, `v`, `+`, `-`, and spaces)
/// within 80 columns; non-structural lines (text annotations) are preserved
/// verbatim, and blank lines collapse to empty.
fn transform_connector(line: &str) -> String {
    let t = line.trim();
    if t.is_empty() {
        return String::new();
    }
    if !is_structural(t) {
        return line.to_string();
    }
    center_in_span(t)
}

fn is_structural(t: &str) -> bool {
    t.bytes()
        .all(|c| matches!(c, b'|' | b'v' | b'+' | b'-' | b' '))
}

/// Centers `content` within `width` columns, putting any odd extra space on the
/// right (matching the table aligner's padding).
fn center_in_width(content: &str, width: usize) -> String {
    let total = width.saturating_sub(disp_width(content));
    let left = total / 2;
    format!(
        "{}{}{}",
        " ".repeat(left),
        content,
        " ".repeat(total - left)
    )
}

/// Left-pads `s` so its content is centered within 80 columns.
fn center_in_span(s: &str) -> String {
    let leading = SPAN.saturating_sub(disp_width(s)) / 2;
    format!("{}{}", " ".repeat(leading), s)
}
