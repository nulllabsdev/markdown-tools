package mdtable

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func readFile(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return string(data)
}

// fixtureNames returns the base name of every *.input fixture in ../testdata.
func fixtureNames(t *testing.T) []string {
	t.Helper()
	inputs, err := filepath.Glob(filepath.Join("..", "testdata", "*.input"))
	if err != nil {
		t.Fatalf("glob fixtures: %v", err)
	}
	if len(inputs) == 0 {
		t.Fatal("no fixtures found in ../testdata")
	}
	names := make([]string, 0, len(inputs))
	for _, in := range inputs {
		names = append(names, strings.TrimSuffix(filepath.Base(in), ".input"))
	}
	return names
}

// TestFormatStringFixtures asserts FormatString(.input) equals .output exactly.
func TestFormatStringFixtures(t *testing.T) {
	for _, name := range fixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			in := readFile(t, filepath.Join("..", "testdata", name+".input"))
			want := readFile(t, filepath.Join("..", "testdata", name+".output"))
			if got := FormatString(in); got != want {
				t.Errorf("mismatch\n--- got ---\n%q\n--- want ---\n%q", got, want)
			}
		})
	}
}

// TestIdempotent asserts formatting already-formatted output is a no-op.
func TestIdempotent(t *testing.T) {
	for _, name := range fixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			out := readFile(t, filepath.Join("..", "testdata", name+".output"))
			if got := FormatString(out); got != out {
				t.Errorf("not idempotent\n--- got ---\n%q\n--- want ---\n%q", got, out)
			}
		})
	}
}

func TestFormatStringPreservesMixedLineEndings(t *testing.T) {
	input := "intro\r\n| A | B |\n| --- | --- |\r\n| x | yy |\nend"
	want := "intro\r\n| A   | B   |\n| --- | --- |\r\n| x   | yy  |\nend"
	if got := FormatString(input); got != want {
		t.Errorf("mixed line endings not preserved\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestCodeFenceCloseRequiresOnlyFenceMarkers(t *testing.T) {
	input := "```markdown\n```not a closer\n| not | touched |\n|-|-|\n```\n| yes | formatted |\n| --- | --- |\n"
	want := "```markdown\n```not a closer\n| not | touched |\n|-|-|\n```\n| yes | formatted |\n| --- | --------- |\n"
	if got := FormatString(input); got != want {
		t.Errorf("code fence closed too early\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

// TestFormatDirectory verifies recursive in-place formatting of *.md files,
// leaving non-markdown files untouched.
func TestFormatDirectory(t *testing.T) {
	dir := t.TempDir()

	mdFixtures := []string{"three-alignments", "cjk"}
	for _, f := range mdFixtures {
		in := readFile(t, filepath.Join("..", "testdata", f+".input"))
		if err := os.WriteFile(filepath.Join(dir, f+".md"), []byte(in), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	// Nested *.md to confirm recursion into subdirectories.
	sub := filepath.Join(dir, "nested")
	if err := os.Mkdir(sub, 0o755); err != nil {
		t.Fatal(err)
	}
	emojiIn := readFile(t, filepath.Join("..", "testdata", "emoji.input"))
	if err := os.WriteFile(filepath.Join(sub, "emoji.md"), []byte(emojiIn), 0o644); err != nil {
		t.Fatal(err)
	}

	// A non-markdown file that must be left exactly as-is.
	const untouched = "| a | b |\n| - | - |\n"
	txt := filepath.Join(dir, "skip.txt")
	if err := os.WriteFile(txt, []byte(untouched), 0o644); err != nil {
		t.Fatal(err)
	}

	changed, err := FormatDirectory(dir)
	if err != nil {
		t.Fatalf("FormatDirectory: %v", err)
	}

	// All three *.md fixtures differ from their formatted form, so each must be
	// reported as changed; the .txt file must not appear.
	wantChanged := map[string]bool{
		filepath.Join(dir, "three-alignments.md"): true,
		filepath.Join(dir, "cjk.md"):              true,
		filepath.Join(sub, "emoji.md"):            true,
	}
	if len(changed) != len(wantChanged) {
		t.Errorf("changed = %v, want %d entries", changed, len(wantChanged))
	}
	for _, p := range changed {
		if !wantChanged[p] {
			t.Errorf("unexpected changed path %q", p)
		}
	}

	for _, f := range mdFixtures {
		want := readFile(t, filepath.Join("..", "testdata", f+".output"))
		if got := readFile(t, filepath.Join(dir, f+".md")); got != want {
			t.Errorf("%s.md not formatted in place\n got: %q\nwant: %q", f, got, want)
		}
	}
	wantEmoji := readFile(t, filepath.Join("..", "testdata", "emoji.output"))
	if got := readFile(t, filepath.Join(sub, "emoji.md")); got != wantEmoji {
		t.Errorf("nested emoji.md not formatted\n got: %q\nwant: %q", got, wantEmoji)
	}
	if got := readFile(t, txt); got != untouched {
		t.Errorf("non-md file modified\n got: %q\nwant: %q", got, untouched)
	}
}
