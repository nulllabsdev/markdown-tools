// Command go-wrap reflows markdown prose paragraphs to a display width.
//
// Usage: go-wrap [-n WIDTH] [path ...]
//
// With no path arguments it reads markdown from stdin and writes the wrapped
// result to stdout. Given one or more paths it wraps each in place: a directory
// is walked recursively for *.md files, and a file is rewritten only if its
// content actually changes. The full path of every changed file is printed, one
// per line. The default width is 80 columns; -n overrides it.
package main

import (
	"flag"
	"fmt"
	"io"
	"os"

	"github.com/nulllabsdev/markdown-tools/go"
)

const defaultWidth = 80

func main() {
	width := flag.Int("n", defaultWidth, "wrap width in display columns")
	flag.Parse()

	if err := run(*width, flag.Args()); err != nil {
		fmt.Fprintln(os.Stderr, "go-wrap:", err)
		os.Exit(1)
	}
}

func run(width int, paths []string) error {
	if width < 1 {
		return fmt.Errorf("width must be at least 1, got %d", width)
	}

	if len(paths) == 0 {
		data, err := io.ReadAll(os.Stdin)
		if err != nil {
			return err
		}
		_, err = io.WriteString(os.Stdout, mdtable.WrapString(string(data), width))
		return err
	}

	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			return err
		}
		if info.IsDir() {
			changed, err := mdtable.WrapDirectory(path, width)
			if err != nil {
				return err
			}
			for _, p := range changed {
				fmt.Println(p)
			}
			continue
		}
		changed, err := wrapFile(path, width, info.Mode().Perm())
		if err != nil {
			return err
		}
		if changed {
			fmt.Println(path)
		}
	}
	return nil
}

// wrapFile rewrites a single file in place, leaving it untouched when the
// wrapped content is identical. It reports whether the file was rewritten.
func wrapFile(path string, width int, perm os.FileMode) (bool, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return false, err
	}
	wrapped := mdtable.WrapString(string(data), width)
	if wrapped == string(data) {
		return false, nil
	}
	if err := os.WriteFile(path, []byte(wrapped), perm); err != nil {
		return false, err
	}
	return true, nil
}
