package mdtable

import (
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// WrapString reflows prose paragraphs in s to width display columns and returns
// the result. It is pure and side-effect free: fenced code blocks, tables,
// headings, lists, blockquotes, thematic breaks, HTML blocks, indented code, and
// YAML front matter pass through untouched, and the original newline style (LF
// vs CRLF) and trailing-newline state are preserved. A width below 1 disables
// wrapping (each paragraph collapses to a single line).
func WrapString(s string, width int) string {
	lines := splitLines(s)

	out := make([]logicalLine, 0, len(lines))
	var para []logicalLine // accumulated prose lines awaiting a flush

	flush := func() {
		if len(para) == 0 {
			return
		}
		out = append(out, wrapParagraph(para, width)...)
		para = para[:0]
	}

	inFence := false
	var fenceMarker byte
	var fenceLen int
	inFrontMatter := false

	for i := 0; i < len(lines); i++ {
		line := lines[i]
		trimmed := strings.TrimSpace(line.text)

		// YAML front matter: a leading `---` on the very first line opens a block
		// that passes through verbatim until its closing `---`/`...`.
		if i == 0 && trimmed == "---" {
			inFrontMatter = true
			out = append(out, line)
			continue
		}
		if inFrontMatter {
			out = append(out, line)
			if trimmed == "---" || trimmed == "..." {
				inFrontMatter = false
			}
			continue
		}

		// Code fences pass through and toggle fence state (reusing the aligner's
		// CommonMark-aware detection).
		if !indentedTooFar(line.text) {
			if !inFence {
				if marker, n, ok := openingFenceToken(trimmed); ok {
					flush()
					inFence, fenceMarker, fenceLen = true, marker, n
					out = append(out, line)
					continue
				}
			} else if isClosingFence(trimmed, fenceMarker, fenceLen) {
				inFence = false
				out = append(out, line)
				continue
			}
		}
		if inFence {
			out = append(out, line)
			continue
		}

		// A pending paragraph immediately followed by a setext underline is a
		// heading; emit both verbatim so the underline keeps matching. (Checked
		// before prose classification because "===" is not otherwise a block.)
		if len(para) > 0 && isSetextUnderline(line.text) {
			out = append(out, para...)
			para = para[:0]
			out = append(out, line)
			continue
		}

		if isProse(line.text) {
			para = append(para, line)
			continue
		}

		// Any non-prose line ends the current paragraph and passes through.
		flush()
		out = append(out, line)
	}
	flush()

	var b strings.Builder
	for _, line := range out {
		b.WriteString(line.text)
		b.WriteString(line.eol)
	}
	return b.String()
}

// WrapDirectory walks root recursively and wraps every *.md file in place to
// width columns, rewriting only files whose content actually changes and
// preserving file mode. It returns the paths of the files it rewrote, in walk
// order.
func WrapDirectory(root string, width int) ([]string, error) {
	var changed []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.EqualFold(filepath.Ext(path), ".md") {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		wrapped := WrapString(string(data), width)
		if wrapped == string(data) {
			return nil
		}
		info, err := d.Info()
		if err != nil {
			return err
		}
		if err := os.WriteFile(path, []byte(wrapped), info.Mode().Perm()); err != nil {
			return err
		}
		changed = append(changed, path)
		return nil
	})
	return changed, err
}

// wrapParagraph greedily packs the words of a prose paragraph into lines no wider
// than width display columns, never splitting a single word. Newly created lines
// reuse the paragraph's first source eol; the final line keeps the paragraph's
// last source eol so trailing-newline and CRLF state survive.
func wrapParagraph(para []logicalLine, width int) []logicalLine {
	var words []string
	for _, line := range para {
		words = append(words, strings.Fields(line.text)...)
	}
	if len(words) == 0 {
		return para
	}

	firstEol := para[0].eol
	lastEol := para[len(para)-1].eol

	var texts []string
	if width < 1 {
		texts = []string{strings.Join(words, " ")}
	} else {
		cur := words[0]
		curW := dispWidth(cur)
		for _, w := range words[1:] {
			ww := dispWidth(w)
			if curW+1+ww <= width {
				cur += " " + w
				curW += 1 + ww
			} else {
				texts = append(texts, cur)
				cur, curW = w, ww
			}
		}
		texts = append(texts, cur)
	}

	wrapped := make([]logicalLine, len(texts))
	for i, text := range texts {
		eol := firstEol
		if i == len(texts)-1 {
			eol = lastEol
		}
		wrapped[i] = logicalLine{text: text, eol: eol}
	}
	return wrapped
}

// isProse reports whether a line is ordinary paragraph text — i.e. it has
// content and is none of the block constructs that must pass through untouched.
// Fenced code, indented code, and front matter are handled by the caller's state
// machine before this is consulted.
func isProse(line string) bool {
	if strings.TrimSpace(line) == "" {
		return false
	}
	if indentedTooFar(line) {
		return false
	}
	if isAtxHeading(line) || isBlockquote(line) || isListItem(line) ||
		isThematicBreak(line) || isHTMLBlockStart(line) || isPipeRow(line) {
		return false
	}
	return true
}

// leadingSpaces returns the count of leading ASCII spaces, capped at the 4 that
// matter for CommonMark block recognition.
func leadingSpaces(line string) int {
	n := 0
	for n < len(line) && n < 4 && line[n] == ' ' {
		n++
	}
	return n
}

// isAtxHeading reports whether a line is an ATX heading (1-6 '#' then a space or
// end of line), indented at most three columns.
func isAtxHeading(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := line[leadingSpaces(line):]
	hashes := 0
	for hashes < len(s) && s[hashes] == '#' {
		hashes++
	}
	if hashes < 1 || hashes > 6 {
		return false
	}
	return hashes == len(s) || s[hashes] == ' ' || s[hashes] == '\t'
}

// isBlockquote reports whether a line begins a block quote (a '>' at most three
// columns in).
func isBlockquote(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := line[leadingSpaces(line):]
	return len(s) > 0 && s[0] == '>'
}

// isListItem reports whether a line begins a bullet or ordered list item.
func isListItem(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := line[leadingSpaces(line):]
	if s == "" {
		return false
	}
	if c := s[0]; c == '-' || c == '*' || c == '+' {
		return len(s) == 1 || s[1] == ' ' || s[1] == '\t'
	}
	digits := 0
	for digits < len(s) && digits < 9 && s[digits] >= '0' && s[digits] <= '9' {
		digits++
	}
	if digits == 0 || digits >= len(s) {
		return false
	}
	if s[digits] != '.' && s[digits] != ')' {
		return false
	}
	rest := digits + 1
	return rest == len(s) || s[rest] == ' ' || s[rest] == '\t'
}

// isThematicBreak reports whether a line is a thematic break: three or more of a
// single '-', '*', or '_' with only spaces between them.
func isThematicBreak(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := strings.TrimSpace(line)
	if len(s) < 3 {
		return false
	}
	marker := s[0]
	if marker != '-' && marker != '*' && marker != '_' {
		return false
	}
	count := 0
	for i := 0; i < len(s); i++ {
		switch s[i] {
		case marker:
			count++
		case ' ':
		default:
			return false
		}
	}
	return count >= 3
}

// isSetextUnderline reports whether a line is a setext heading underline: a run
// of only '=' or only '-', indented at most three columns.
func isSetextUnderline(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := strings.TrimRight(line[leadingSpaces(line):], " \t")
	if s == "" {
		return false
	}
	marker := s[0]
	if marker != '=' && marker != '-' {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] != marker {
			return false
		}
	}
	return true
}

// isHTMLBlockStart reports whether a line begins an HTML block (a '<' at most
// three columns in). Such lines are left untouched.
func isHTMLBlockStart(line string) bool {
	if indentedTooFar(line) {
		return false
	}
	s := line[leadingSpaces(line):]
	return len(s) > 0 && s[0] == '<'
}
