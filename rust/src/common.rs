use std::path::Path;

use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy)]
pub(crate) struct LogicalLine<'a> {
    pub(crate) text: &'a str,
    pub(crate) eol: &'a str,
}

pub(crate) fn disp_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub(crate) fn split_lines(input: &str) -> Vec<LogicalLine<'_>> {
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

pub(crate) fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

pub(crate) fn indented_too_far(line: &str) -> bool {
    let bytes = line.as_bytes();
    let spaces = bytes.iter().take_while(|&&b| b == b' ').count();
    spaces >= 4 || bytes.get(spaces) == Some(&b'\t')
}

pub(crate) fn opening_fence_token(trimmed: &str) -> Option<(u8, usize)> {
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
    if c == b'`' && bytes[n..].contains(&b'`') {
        return None;
    }
    Some((c, n))
}

pub(crate) fn is_closing_fence(trimmed: &str, marker: u8, min_len: usize) -> bool {
    trimmed.len() >= min_len && trimmed.bytes().all(|b| b == marker)
}

pub(crate) fn is_pipe_row(line: &str) -> bool {
    let t = line.trim().as_bytes();
    t.len() >= 2 && t[0] == b'|' && t[t.len() - 1] == b'|'
}
