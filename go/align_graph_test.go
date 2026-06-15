package mdtable

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// graphFixtureNames returns the base name of every *.input fixture in
// ../testdata/graph.
func graphFixtureNames(t *testing.T) []string {
	t.Helper()
	inputs, err := filepath.Glob(filepath.Join("..", "testdata", "graph", "*.input"))
	if err != nil {
		t.Fatalf("glob graph fixtures: %v", err)
	}
	if len(inputs) == 0 {
		t.Fatal("no graph fixtures found in ../testdata/graph")
	}
	names := make([]string, 0, len(inputs))
	for _, in := range inputs {
		names = append(names, strings.TrimSuffix(filepath.Base(in), ".input"))
	}
	return names
}

// TestAlignGraphFixtures asserts AlignGraphString(.input) equals .output exactly.
func TestAlignGraphFixtures(t *testing.T) {
	for _, name := range graphFixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			in := readFile(t, filepath.Join("..", "testdata", "graph", name+".input"))
			want := readFile(t, filepath.Join("..", "testdata", "graph", name+".output"))
			if got := AlignGraphString(in); got != want {
				t.Errorf("mismatch\n--- got ---\n%q\n--- want ---\n%q", got, want)
			}
		})
	}
}

// TestAlignGraphIdempotent asserts re-running on aligned output is a no-op.
func TestAlignGraphIdempotent(t *testing.T) {
	for _, name := range graphFixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			out := readFile(t, filepath.Join("..", "testdata", "graph", name+".output"))
			if got := AlignGraphString(out); got != out {
				t.Errorf("not idempotent\n--- got ---\n%q\n--- want ---\n%q", got, out)
			}
		})
	}
}

func TestAlignGraphSingleBoxNormalizes(t *testing.T) {
	input := "```text\n+--------+\n|  Hi    |\n+--------+\n```\n"
	want := "```text\n" +
		"                        +------------------------------+\n" +
		"                        |              Hi              |\n" +
		"                        +------------------------------+\n" +
		"```\n"
	if got := AlignGraphString(input); got != want {
		t.Errorf("single box not normalized\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestAlignGraphNonGraphTextUntouched(t *testing.T) {
	input := "```text\njust prose\nno boxes\n```\n"
	if got := AlignGraphString(input); got != input {
		t.Errorf("non-graph text block modified\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestAlignGraphNonTextFenceUntouched(t *testing.T) {
	input := "```go\n+------+\n| code |\n+------+\n```\n"
	if got := AlignGraphString(input); got != input {
		t.Errorf("non-text fence modified\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestAlignGraphUnfencedUntouched(t *testing.T) {
	input := "text with | pipes and +--+ but no fence\n"
	if got := AlignGraphString(input); got != input {
		t.Errorf("unfenced markdown modified\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestAlignGraphConnectorTextPreserved(t *testing.T) {
	// A non-structural annotation between connectors keeps its exact indentation;
	// the structural `|`/`v` lines re-center to column 39.
	input := "```text\n" +
		"                        +------------------------------+\n" +
		"                        |             A                |\n" +
		"                        +------------------------------+\n" +
		"   |\n" +
		"          note\n" +
		"   v\n" +
		"                        +------------------------------+\n" +
		"                        |             B                |\n" +
		"                        +------------------------------+\n" +
		"```\n"
	got := AlignGraphString(input)
	if !strings.Contains(got, "                                       |\n") {
		t.Errorf("structural connector not centered to col 39:\n%q", got)
	}
	if !strings.Contains(got, "          note\n") {
		t.Errorf("annotation indentation not preserved:\n%q", got)
	}
}

func TestAlignGraphPreservesCRLF(t *testing.T) {
	input := "```text\r\n+----+\r\n| X  |\r\n+----+\r\n```\r\n"
	got := AlignGraphString(input)
	if !strings.HasPrefix(got, "```text\r\n") || !strings.HasSuffix(got, "```\r\n") {
		t.Errorf("CRLF not preserved:\n%q", got)
	}
	if strings.Contains(got, "\r\r") || strings.Count(got, "\n") != strings.Count(got, "\r\n") {
		t.Errorf("inconsistent line endings:\n%q", got)
	}
}

func TestAlignGraphDirectory(t *testing.T) {
	dir := t.TempDir()

	graph := readFile(t, filepath.Join("..", "testdata", "graph", "malformed-box.input"))
	want := readFile(t, filepath.Join("..", "testdata", "graph", "malformed-box.output"))
	if err := os.WriteFile(filepath.Join(dir, "doc.md"), []byte(graph), 0o644); err != nil {
		t.Fatal(err)
	}
	// Already-canonical graph: unchanged, so not reported.
	canonical := readFile(t, filepath.Join("..", "testdata", "graph", "single-box.input"))
	if err := os.WriteFile(filepath.Join(dir, "ok.md"), []byte(canonical), 0o644); err != nil {
		t.Fatal(err)
	}
	// Non-markdown file left as-is.
	const untouched = "```text\n+--+\n|a |\n+--+\n```\n"
	txt := filepath.Join(dir, "skip.txt")
	if err := os.WriteFile(txt, []byte(untouched), 0o644); err != nil {
		t.Fatal(err)
	}

	changed, err := AlignGraphDirectory(dir)
	if err != nil {
		t.Fatalf("AlignGraphDirectory: %v", err)
	}
	if len(changed) != 1 || changed[0] != filepath.Join(dir, "doc.md") {
		t.Errorf("changed = %v, want [doc.md]", changed)
	}
	if got := readFile(t, filepath.Join(dir, "doc.md")); got != want {
		t.Errorf("doc.md not aligned in place\n got: %q\nwant: %q", got, want)
	}
	if got := readFile(t, txt); got != untouched {
		t.Errorf("non-md file modified\n got: %q", got)
	}
}
