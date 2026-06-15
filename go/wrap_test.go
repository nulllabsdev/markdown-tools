package mdtable

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// wrapFixtureNames returns the base name of every *.input fixture in
// ../testdata/wrap.
func wrapFixtureNames(t *testing.T) []string {
	t.Helper()
	inputs, err := filepath.Glob(filepath.Join("..", "testdata", "wrap", "*.input"))
	if err != nil {
		t.Fatalf("glob wrap fixtures: %v", err)
	}
	if len(inputs) == 0 {
		t.Fatal("no wrap fixtures found in ../testdata/wrap")
	}
	names := make([]string, 0, len(inputs))
	for _, in := range inputs {
		names = append(names, strings.TrimSuffix(filepath.Base(in), ".input"))
	}
	return names
}

// TestWrapStringFixtures asserts WrapString(.input, 80) equals .output exactly.
func TestWrapStringFixtures(t *testing.T) {
	for _, name := range wrapFixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			in := readFile(t, filepath.Join("..", "testdata", "wrap", name+".input"))
			want := readFile(t, filepath.Join("..", "testdata", "wrap", name+".output"))
			if got := WrapString(in, 80); got != want {
				t.Errorf("mismatch\n--- got ---\n%q\n--- want ---\n%q", got, want)
			}
		})
	}
}

// TestWrapIdempotent asserts wrapping already-wrapped output is a no-op.
func TestWrapIdempotent(t *testing.T) {
	for _, name := range wrapFixtureNames(t) {
		t.Run(name, func(t *testing.T) {
			out := readFile(t, filepath.Join("..", "testdata", "wrap", name+".output"))
			if got := WrapString(out, 80); got != out {
				t.Errorf("not idempotent\n--- got ---\n%q\n--- want ---\n%q", got, out)
			}
		})
	}
}

func TestWrapCustomWidth(t *testing.T) {
	input := "alpha beta gamma delta epsilon zeta eta theta\n"
	want := "alpha beta gamma\ndelta epsilon\nzeta eta theta\n"
	if got := WrapString(input, 16); got != want {
		t.Errorf("custom width\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestWrapLongWordNeverSplit(t *testing.T) {
	input := "see https://example.com/a/very/long/path/that/exceeds/the/width now\n"
	want := "see\nhttps://example.com/a/very/long/path/that/exceeds/the/width\nnow\n"
	if got := WrapString(input, 20); got != want {
		t.Errorf("long word split\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestWrapLeavesBlocksUntouched(t *testing.T) {
	input := "# A heading that is quite long but must never ever be wrapped by the tool here\n\n" +
		"- a list item that is also long enough to exceed the configured wrap width easily\n\n" +
		"> a block quote line that is long enough to exceed the width but stays as written\n\n" +
		"| col | another column header that is long |\n| --- | --- |\n\n" +
		"```\nthis fenced code line is long enough to exceed the width but must be left alone\n```\n"
	if got := WrapString(input, 40); got != input {
		t.Errorf("block content was modified\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestWrapPreservesSetextHeading(t *testing.T) {
	input := "A setext heading whose text is long enough to exceed the chosen wrap width here\n===\n\nbody\n"
	if got := WrapString(input, 40); got != input {
		t.Errorf("setext heading wrapped\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestWrapPreservesFrontMatter(t *testing.T) {
	input := "---\ntitle: A very long front matter value that should never be wrapped by the tool\n---\n\nbody\n"
	if got := WrapString(input, 40); got != input {
		t.Errorf("front matter wrapped\n--- got ---\n%q\n--- want ---\n%q", got, input)
	}
}

func TestWrapPreservesMixedLineEndings(t *testing.T) {
	input := "one two three four five\r\nsix seven eight nine ten\n"
	want := "one two three\r\nfour five six\r\nseven eight\r\nnine ten\n"
	if got := WrapString(input, 14); got != want {
		t.Errorf("line endings not preserved\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestWrapDirectory(t *testing.T) {
	dir := t.TempDir()

	long := "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi\n"
	if err := os.WriteFile(filepath.Join(dir, "doc.md"), []byte(long), 0o644); err != nil {
		t.Fatal(err)
	}
	sub := filepath.Join(dir, "nested")
	if err := os.Mkdir(sub, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(sub, "deep.md"), []byte(long), 0o644); err != nil {
		t.Fatal(err)
	}
	// Already short prose: unchanged, so it must not be reported.
	if err := os.WriteFile(filepath.Join(dir, "short.md"), []byte("hi there\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	// Non-markdown file must be left exactly as-is.
	const untouched = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi\n"
	txt := filepath.Join(dir, "skip.txt")
	if err := os.WriteFile(txt, []byte(untouched), 0o644); err != nil {
		t.Fatal(err)
	}

	changed, err := WrapDirectory(dir, 80)
	if err != nil {
		t.Fatalf("WrapDirectory: %v", err)
	}

	wantChanged := map[string]bool{
		filepath.Join(dir, "doc.md"):  true,
		filepath.Join(sub, "deep.md"): true,
	}
	if len(changed) != len(wantChanged) {
		t.Errorf("changed = %v, want %d entries", changed, len(wantChanged))
	}
	for _, p := range changed {
		if !wantChanged[p] {
			t.Errorf("unexpected changed path %q", p)
		}
	}
	if got := readFile(t, txt); got != untouched {
		t.Errorf("non-md file modified\n got: %q\nwant: %q", got, untouched)
	}
}
