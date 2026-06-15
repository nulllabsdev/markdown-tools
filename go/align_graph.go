package mdtable

import (
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// graphInnerWidth is the fixed inner width of every box (between the `|`s).
const graphInnerWidth = 30

// graphSpan is the column span graphs are centered within.
const graphSpan = 80

// graphBoxGap is the number of spaces between side-by-side boxes in a row.
const graphBoxGap = 2

// AlignGraphString re-renders ASCII flowcharts inside fenced ```text blocks to a
// canonical form: every box is normalised to a 30-character inner width with its
// label centered, each box-row is centered within 80 columns, and structural
// connector lines (made only of `|`, `v`, `+`, `-`) are re-centered to line up
// beneath the boxes. It is pure and side-effect free. Non-graph ```text blocks,
// non-`text` fences, and all unfenced markdown pass through untouched, as do the
// original newline style and trailing-newline state.
func AlignGraphString(s string) string {
	lines := splitLines(s)

	out := make([]logicalLine, 0, len(lines))
	for i := 0; i < len(lines); {
		trimmed := strings.TrimSpace(lines[i].text)

		marker, n, ok := byte(0), 0, false
		if !indentedTooFar(lines[i].text) {
			marker, n, ok = openingFenceToken(trimmed)
		}
		if !ok {
			out = append(out, lines[i])
			i++
			continue
		}

		// Opening fence: gather the block up to its closing fence.
		info := strings.TrimSpace(trimmed[n:])
		j := i + 1
		for j < len(lines) {
			tj := strings.TrimSpace(lines[j].text)
			if !indentedTooFar(lines[j].text) && isClosingFence(tj, marker, n) {
				break
			}
			j++
		}

		out = append(out, lines[i]) // opener, verbatim
		inner := lines[i+1 : j]
		if info == "text" {
			out = append(out, alignGraphBlock(inner)...)
		} else {
			out = append(out, inner...)
		}
		if j < len(lines) {
			out = append(out, lines[j]) // closer, verbatim
			i = j + 1
		} else {
			i = j // unterminated fence
		}
	}

	var b strings.Builder
	for _, line := range out {
		b.WriteString(line.text)
		b.WriteString(line.eol)
	}
	return b.String()
}

// AlignGraphDirectory walks root recursively and re-renders graphs in every *.md
// file in place, rewriting only files whose content actually changes and
// preserving file mode. It returns the paths of the files it rewrote, in walk
// order.
func AlignGraphDirectory(root string) ([]string, error) {
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
		formatted := AlignGraphString(string(data))
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

// alignGraphBlock re-renders the inner lines of a ```text block if they form a
// graph (contain at least one box-border row); otherwise the block is returned
// unchanged. A block is also left untouched if any box label would overflow the
// 30-column inner width.
func alignGraphBlock(inner []logicalLine) []logicalLine {
	texts := make([]string, len(inner))
	for i, l := range inner {
		texts[i] = l.text
	}
	if !containsBox(texts) {
		return inner
	}

	eol := "\n"
	if len(inner) > 0 {
		eol = inner[0].eol
	}

	rendered, ok := renderGraph(texts)
	if !ok {
		return inner // a label overflowed; leave the block byte-identical
	}
	out := make([]logicalLine, len(rendered))
	for i, text := range rendered {
		out[i] = logicalLine{text: text, eol: eol}
	}
	return out
}

// renderGraph rebuilds every line of a graph block. It returns ok=false if any
// box label exceeds the inner width, signalling the caller to leave the block
// untouched.
func renderGraph(texts []string) ([]string, bool) {
	out := make([]string, 0, len(texts))
	for i := 0; i < len(texts); {
		if isBoxBorderRow(texts[i]) {
			segs := boxSegments(texts[i])
			j := i + 1
			for j < len(texts) && !isBoxBorderRow(texts[j]) {
				j++
			}
			rows, ok := renderBoxRow(segs, texts[i+1:j])
			if !ok {
				return nil, false
			}
			out = append(out, rows...)
			if j < len(texts) {
				i = j + 1 // consume the bottom border
			} else {
				i = j
			}
			continue
		}
		out = append(out, transformConnector(texts[i]))
		i++
	}
	return out, true
}

// containsBox reports whether any line is a box-border row.
func containsBox(texts []string) bool {
	for _, t := range texts {
		if isBoxBorderRow(t) {
			return true
		}
	}
	return false
}

// isBoxBorderRow reports whether a line is one or more box borders side by side:
// each 2-space-separated segment is `+`, one or more `-`, `+` (so a branch line
// like `+----+----+`, which has an interior `+`, is NOT a border).
func isBoxBorderRow(line string) bool {
	t := strings.TrimSpace(line)
	if t == "" {
		return false
	}
	for _, part := range strings.Split(t, "  ") {
		part = strings.TrimSpace(part)
		if !isBoxSegment(part) {
			return false
		}
	}
	return true
}

func isBoxSegment(part string) bool {
	if len(part) < 3 || part[0] != '+' || part[len(part)-1] != '+' {
		return false
	}
	for i := 1; i < len(part)-1; i++ {
		if part[i] != '-' {
			return false
		}
	}
	return true
}

// boxSegments returns the [start,end] byte-column range of each box on a border
// line (start at the leading `+`, end at the trailing `+`).
func boxSegments(line string) [][2]int {
	var segs [][2]int
	i := 0
	for i < len(line) {
		if line[i] != '+' {
			i++
			continue
		}
		start := i
		i++
		for i < len(line) && line[i] == '-' {
			i++
		}
		if i < len(line) && line[i] == '+' {
			segs = append(segs, [2]int{start, i})
			i++
		}
	}
	return segs
}

// renderBoxRow normalises one row of boxes. segs gives the input column ranges
// (used only to slice labels out of the content lines); the output geometry is
// canonical: 30-inner boxes joined by two spaces and centered within 80 columns.
func renderBoxRow(segs [][2]int, content []string) ([]string, bool) {
	nBoxes := len(segs)
	if nBoxes == 0 {
		return nil, true
	}

	// labels[b] holds the label for each content line of box b.
	labels := make([][]string, nBoxes)
	for b, seg := range segs {
		for _, line := range content {
			labels[b] = append(labels[b], cellLabel(line, seg[0], seg[1]))
		}
	}
	for b := range labels {
		for _, label := range labels[b] {
			if dispWidth(label) > graphInnerWidth {
				return nil, false
			}
		}
	}

	border := "+" + strings.Repeat("-", graphInnerWidth) + "+"
	borders := make([]string, nBoxes)
	for b := range borders {
		borders[b] = border
	}
	gap := strings.Repeat(" ", graphBoxGap)

	rows := make([]string, 0, len(content)+2)
	rows = append(rows, strings.Join(borders, gap))
	for r := 0; r < len(content); r++ {
		cells := make([]string, nBoxes)
		for b := range cells {
			label := ""
			if r < len(labels[b]) {
				label = labels[b][r]
			}
			cells[b] = "|" + centerInWidth(label, graphInnerWidth) + "|"
		}
		rows = append(rows, strings.Join(cells, gap))
	}
	rows = append(rows, strings.Join(borders, gap))

	for i, row := range rows {
		rows[i] = centerInSpan(row)
	}
	return rows, true
}

// cellLabel extracts and trims the label of a box cell occupying byte columns
// [start,end] of a content line.
func cellLabel(line string, start, end int) string {
	if start >= len(line) {
		return ""
	}
	if end >= len(line) {
		end = len(line) - 1
	}
	cell := line[start : end+1]
	cell = strings.TrimPrefix(cell, "|")
	cell = strings.TrimSuffix(cell, "|")
	return strings.TrimSpace(cell)
}

// transformConnector re-centers a structural connector line (only `|`, `v`, `+`,
// `-`, and spaces) within 80 columns; non-structural lines (text annotations)
// are preserved verbatim, and blank lines collapse to empty.
func transformConnector(line string) string {
	t := strings.TrimSpace(line)
	if t == "" {
		return ""
	}
	if !isStructural(t) {
		return line
	}
	return centerInSpan(t)
}

func isStructural(t string) bool {
	for i := 0; i < len(t); i++ {
		switch t[i] {
		case '|', 'v', '+', '-', ' ':
		default:
			return false
		}
	}
	return true
}

// centerInWidth centers content within width columns, putting any odd extra
// space on the right (matching the table aligner's padding).
func centerInWidth(content string, width int) string {
	total := width - dispWidth(content)
	if total < 0 {
		total = 0
	}
	left := total / 2
	return strings.Repeat(" ", left) + content + strings.Repeat(" ", total-left)
}

// centerInSpan left-pads s so its trimmed content is centered within 80 columns.
func centerInSpan(s string) string {
	leading := (graphSpan - dispWidth(s)) / 2
	if leading < 0 {
		leading = 0
	}
	return strings.Repeat(" ", leading) + s
}
