package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestRunUnknownSubcommand(t *testing.T) {
	var out, err bytes.Buffer
	code := run([]string{"wat"}, bytes.NewBufferString(""), &out, &err)
	if code != 1 {
		t.Fatalf("code = %d, want 1", code)
	}
	if err.Len() == 0 {
		t.Fatal("expected usage on stderr")
	}
}

func TestRunAllStdin(t *testing.T) {
	input := "| a | bb |\n|---|---|\n| 1 | 2 |\n\nalpha beta gamma delta epsilon\n"
	var out, err bytes.Buffer
	code := run([]string{"all", "-n", "12"}, bytes.NewBufferString(input), &out, &err)
	if code != 0 {
		t.Fatalf("code = %d, stderr = %q", code, err.String())
	}
	want := "| a   | bb  |\n| --- | --- |\n| 1   | 2   |\n\nalpha beta\ngamma delta\nepsilon\n"
	if got := out.String(); got != want {
		t.Fatalf("stdout mismatch\n--- got ---\n%q\n--- want ---\n%q", got, want)
	}
}

func TestRunAllDirectoryReportsSortedUniquePaths(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "doc.md")
	input := "```text\n+----+\n| X  |\n+----+\n```\n\nalpha beta gamma delta\n"
	if err := os.WriteFile(file, []byte(input), 0o644); err != nil {
		t.Fatal(err)
	}

	var out, err bytes.Buffer
	code := run([]string{"all", dir}, bytes.NewBufferString(""), &out, &err)
	if code != 0 {
		t.Fatalf("code = %d, stderr = %q", code, err.String())
	}
	want := file + "\n"
	if got := out.String(); got != want {
		t.Fatalf("stdout = %q, want %q", got, want)
	}
}
