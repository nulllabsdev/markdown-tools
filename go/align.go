// Package mdtable aligns the columns of GitHub-style pipe tables in markdown so
// that they read cleanly as raw text. See README.md for the full specification.
package mdtable

import (
	"io/fs"
	"os"
	"path/filepath"
	"strings"

	"github.com/mattn/go-runewidth"
)

// minColWidth is the smallest column width the formatter will emit, so that
// every separator form (e.g. ":-:") stays valid even for one-character columns.
const minColWidth = 3

// widthCond measures display width deterministically: ambiguous-width runes are
// treated as narrow regardless of the LANG environment, while CJK characters and
// emoji remain width 2.
var widthCond = &runewidth.Condition{EastAsianWidth: false}

func dispWidth(s string) int { return widthCond.StringWidth(s) }

// align describes how a column's cells are positioned. alignDefault and
// alignLeft both left-align content; they differ only in how the separator row
// is rendered (plain dashes vs. a leading colon).
type align int

const (
	alignDefault align = iota
	alignLeft
	alignCenter
	alignRight
)

type logicalLine struct {
	text string
	eol  string
}

// FormatString formats every recognised pipe table in s and returns the result.
// It is pure and side-effect free. The original newline style (LF vs CRLF) and
// trailing-newline state are preserved.
func FormatString(s string) string {
	lines := splitLines(s)

	out := make([]logicalLine, 0, len(lines))
	inFence := false
	var fenceMarker byte
	var fenceLen int

	for i := 0; i < len(lines); {
		trimmed := strings.TrimSpace(lines[i].text)

		// Code-fence boundaries pass through and toggle fence state.
		if marker, n, ok := fenceToken(trimmed); ok {
			if !inFence {
				inFence, fenceMarker, fenceLen = true, marker, n
			} else if marker == fenceMarker && n >= fenceLen {
				inFence = false
			}
			out = append(out, lines[i])
			i++
			continue
		}
		if inFence {
			out = append(out, lines[i])
			i++
			continue
		}

		// A table is a pipe row immediately followed by a valid separator row.
		if i+1 < len(lines) && isPipeRow(lines[i].text) && isSeparatorRow(lines[i+1].text) {
			j := i + 2
			for j < len(lines) && isPipeRow(lines[j].text) {
				j++
			}
			formatted := formatTable(lineTexts(lines[i:j]))
			for n, text := range formatted {
				out = append(out, logicalLine{text: text, eol: lines[i+n].eol})
			}
			i = j
			continue
		}

		out = append(out, lines[i])
		i++
	}

	var result strings.Builder
	for _, line := range out {
		result.WriteString(line.text)
		result.WriteString(line.eol)
	}
	return result.String()
}

func splitLines(s string) []logicalLine {
	if s == "" {
		return []logicalLine{{}}
	}

	lines := make([]logicalLine, 0, strings.Count(s, "\n")+1)
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] != '\n' {
			continue
		}
		end := i
		eol := "\n"
		if i > start && s[i-1] == '\r' {
			end = i - 1
			eol = "\r\n"
		}
		lines = append(lines, logicalLine{text: s[start:end], eol: eol})
		start = i + 1
	}
	if start < len(s) {
		lines = append(lines, logicalLine{text: s[start:]})
	}
	return lines
}

func lineTexts(lines []logicalLine) []string {
	texts := make([]string, len(lines))
	for i, line := range lines {
		texts[i] = line.text
	}
	return texts
}

// FormatDirectory walks root recursively and formats every *.md file in place,
// rewriting only files whose content actually changes and preserving file mode.
// It returns the paths of the files it rewrote, in walk order.
func FormatDirectory(root string) ([]string, error) {
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
		formatted := FormatString(string(data))
		if formatted == string(data) {
			return nil
		}
		info, err := d.Info()
		if err != nil {
			return err
		}
		if err := os.WriteFile(path, []byte(formatted), info.Mode().Perm()); err != nil {
			return err
		}
		changed = append(changed, path)
		return nil
	})
	return changed, err
}

// fenceToken reports whether a trimmed line opens or closes a fenced code block,
// returning the fence character and run length.
func fenceToken(trimmed string) (byte, int, bool) {
	if len(trimmed) < 3 {
		return 0, 0, false
	}
	c := trimmed[0]
	if c != '`' && c != '~' {
		return 0, 0, false
	}
	n := 0
	for n < len(trimmed) && trimmed[n] == c {
		n++
	}
	if n < 3 {
		return 0, 0, false
	}
	return c, n, true
}

// isPipeRow reports whether a line's trimmed text starts and ends with '|'.
func isPipeRow(line string) bool {
	t := strings.TrimSpace(line)
	return len(t) >= 2 && t[0] == '|' && t[len(t)-1] == '|'
}

// isSeparatorRow reports whether a line is a pipe row whose every cell is a
// valid alignment separator (optional colons around one or more dashes).
func isSeparatorRow(line string) bool {
	if !isPipeRow(line) {
		return false
	}
	cells := splitCells(line)
	for _, c := range cells {
		if !isSeparatorCell(c) {
			return false
		}
	}
	return len(cells) > 0
}

func isSeparatorCell(c string) bool {
	i := 0
	if i < len(c) && c[i] == ':' {
		i++
	}
	dashes := 0
	for i < len(c) && c[i] == '-' {
		i, dashes = i+1, dashes+1
	}
	if dashes == 0 {
		return false
	}
	if i < len(c) && c[i] == ':' {
		i++
	}
	return i == len(c)
}

// splitCells strips the leading and trailing pipe of a pipe row and splits the
// remainder on unescaped '|', trimming surrounding spaces from each cell. An
// escaped pipe (\|) is preserved as literal content. Iterating bytes is safe:
// '|' and '\' are ASCII and never appear inside a multibyte UTF-8 sequence.
func splitCells(line string) []string {
	t := strings.TrimSpace(line)
	t = t[1 : len(t)-1]

	var cells []string
	var b strings.Builder
	for i := 0; i < len(t); i++ {
		ch := t[i]
		if ch == '\\' && i+1 < len(t) {
			b.WriteByte(ch)
			b.WriteByte(t[i+1])
			i++
			continue
		}
		if ch == '|' {
			cells = append(cells, strings.TrimSpace(b.String()))
			b.Reset()
			continue
		}
		b.WriteByte(ch)
	}
	cells = append(cells, strings.TrimSpace(b.String()))
	return cells
}

// formatTable formats a table block: header (block[0]), separator (block[1]),
// and zero or more body rows.
func formatTable(block []string) []string {
	header := splitCells(block[0])
	sep := splitCells(block[1])
	body := make([][]string, 0, len(block)-2)
	for _, row := range block[2:] {
		body = append(body, splitCells(row))
	}

	numCols := len(header)
	if len(sep) > numCols {
		numCols = len(sep)
	}
	for _, r := range body {
		if len(r) > numCols {
			numCols = len(r)
		}
	}

	aligns := make([]align, numCols)
	for c := range aligns {
		if c < len(sep) {
			aligns[c] = parseAlign(sep[c])
		}
	}

	// Column widths come from header and body cells only (not the separator),
	// floored at the minimum width.
	widths := make([]int, numCols)
	for c := range widths {
		widths[c] = minColWidth
	}
	consider := func(cells []string) {
		for c, cell := range cells {
			if w := dispWidth(cell); w > widths[c] {
				widths[c] = w
			}
		}
	}
	consider(header)
	for _, r := range body {
		consider(r)
	}

	out := make([]string, 0, len(block))
	out = append(out, renderRow(header, widths, aligns))
	out = append(out, renderSeparator(widths, aligns))
	for _, r := range body {
		out = append(out, renderRow(r, widths, aligns))
	}
	return out
}

func parseAlign(sepCell string) align {
	left := strings.HasPrefix(sepCell, ":")
	right := strings.HasSuffix(sepCell, ":")
	switch {
	case left && right:
		return alignCenter
	case right:
		return alignRight
	case left:
		return alignLeft
	default:
		return alignDefault
	}
}

func renderRow(cells []string, widths []int, aligns []align) string {
	fields := make([]string, len(widths))
	for c := range fields {
		content := ""
		if c < len(cells) {
			content = cells[c]
		}
		fields[c] = pad(content, widths[c], aligns[c])
	}
	return "| " + strings.Join(fields, " | ") + " |"
}

// pad positions content within width display columns. Spaces are width 1, so
// the space count equals the width deficit. Centring puts any odd extra space
// on the right.
func pad(content string, width int, a align) string {
	total := width - dispWidth(content)
	if total < 0 {
		total = 0
	}
	switch a {
	case alignRight:
		return strings.Repeat(" ", total) + content
	case alignCenter:
		left := total / 2
		return strings.Repeat(" ", left) + content + strings.Repeat(" ", total-left)
	default: // alignDefault, alignLeft
		return content + strings.Repeat(" ", total)
	}
}

func renderSeparator(widths []int, aligns []align) string {
	fields := make([]string, len(widths))
	for c := range fields {
		fields[c] = sepField(widths[c], aligns[c])
	}
	return "| " + strings.Join(fields, " | ") + " |"
}

func sepField(width int, a align) string {
	switch a {
	case alignLeft:
		return ":" + strings.Repeat("-", width-1)
	case alignRight:
		return strings.Repeat("-", width-1) + ":"
	case alignCenter:
		return ":" + strings.Repeat("-", width-2) + ":"
	default:
		return strings.Repeat("-", width)
	}
}
