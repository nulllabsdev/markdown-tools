// Command go-align aligns the columns of GitHub-style pipe tables in markdown.
//
// With no arguments it reads markdown from stdin and writes the formatted
// result to stdout. Given one or more paths it formats each in place: a
// directory is walked recursively for *.md files, and a file is rewritten only
// if its content actually changes.
package main

import (
	"fmt"
	"io"
	"os"

	"github.com/nulllabsdev/markdown-tools/go"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "go-align:", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		data, err := io.ReadAll(os.Stdin)
		if err != nil {
			return err
		}
		_, err = io.WriteString(os.Stdout, mdtable.FormatString(string(data)))
		return err
	}

	for _, path := range args {
		info, err := os.Stat(path)
		if err != nil {
			return err
		}
		if info.IsDir() {
			if err := mdtable.FormatDirectory(path); err != nil {
				return err
			}
			continue
		}
		if err := formatFile(path, info.Mode().Perm()); err != nil {
			return err
		}
	}
	return nil
}

// formatFile rewrites a single file in place, leaving it untouched when the
// formatted content is identical.
func formatFile(path string, perm os.FileMode) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	formatted := mdtable.FormatString(string(data))
	if formatted == string(data) {
		return nil
	}
	return os.WriteFile(path, []byte(formatted), perm)
}
