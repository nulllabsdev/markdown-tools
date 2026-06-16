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

func TestRunVersion(t *testing.T) {
	origBuild := versionBuild
	versionBuild = "v1.2.3-4-gabc1234"
	t.Cleanup(func() {
		versionBuild = origBuild
	})

	var out, err bytes.Buffer
	code := run([]string{"-v"}, bytes.NewBufferString(""), &out, &err)
	if code != 0 {
		t.Fatalf("code = %d, stderr = %q", code, err.String())
	}
	if got := out.String(); got != "build v1.2.3-4-gabc1234\n\n" {
		t.Fatalf("stdout = %q, want %q", got, "build v1.2.3-4-gabc1234\n\n")
	}
	if err.Len() != 0 {
		t.Fatalf("stderr = %q, want empty", err.String())
	}
}

func TestRunVersionExactTagPrintsSingleLine(t *testing.T) {
	origBuild := versionBuild
	versionBuild = "v1.2.3"
	t.Cleanup(func() { versionBuild = origBuild })

	var out bytes.Buffer
	code := run([]string{"-v"}, bytes.NewBufferString(""), &out, &bytes.Buffer{})
	if code != 0 {
		t.Fatalf("code = %d", code)
	}
	if got := out.String(); got != "build v1.2.3\n\n" {
		t.Fatalf("stdout = %q, want %q", got, "build v1.2.3\n\n")
	}
}

func TestRunAllStdin(t *testing.T) {
	origBuild := versionBuild
	versionBuild = "v1.2.3-4-gabc1234"
	t.Cleanup(func() { versionBuild = origBuild })

	input := "| a | bb |\n|---|---|\n| 1 | 2 |\n\nalpha beta gamma delta epsilon\n"
	var out, err bytes.Buffer
	code := run([]string{"all", "-n", "12"}, bytes.NewBufferString(input), &out, &err)
	if code != 0 {
		t.Fatalf("code = %d, stderr = %q", code, err.String())
	}
	want := "build v1.2.3-4-gabc1234\n| a   | bb  |\n| --- | --- |\n| 1   | 2   |\n\nalpha beta\ngamma delta\nepsilon\n\n"
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
	origBuild := versionBuild
	versionBuild = "v1.2.3-4-gabc1234"
	t.Cleanup(func() { versionBuild = origBuild })

	code := run([]string{"all", dir}, bytes.NewBufferString(""), &out, &err)
	if code != 0 {
		t.Fatalf("code = %d, stderr = %q", code, err.String())
	}
	want := "build v1.2.3-4-gabc1234\n" + file + "\n\n"
	if got := out.String(); got != want {
		t.Fatalf("stdout = %q, want %q", got, want)
	}
}
