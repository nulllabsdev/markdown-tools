// Command go-align-graph re-renders ASCII flowcharts inside fenced ```text
// blocks to a canonical centered form.
//
// With no arguments it reads markdown from stdin and writes the result to
// stdout. Given one or more paths it rewrites each in place: a directory is
// walked recursively for *.md files, and a file is rewritten only if its content
// actually changes. The full path of every changed file is printed, one per
// line.
package main

import (
	"fmt"
	"io"
	"os"

	"github.com/nulllabsdev/markdown-tools/go"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "go-align-graph:", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		data, err := io.ReadAll(os.Stdin)
		if err != nil {
			return err
		}
		_, err = io.WriteString(os.Stdout, mdtable.AlignGraphString(string(data)))
		return err
	}

	for _, path := range args {
		info, err := os.Stat(path)
		if err != nil {
			return err
		}
		if info.IsDir() {
			changed, err := mdtable.AlignGraphDirectory(path)
			if err != nil {
				return err
			}
			for _, p := range changed {
				fmt.Println(p)
			}
			continue
		}
		changed, err := alignFile(path, info.Mode().Perm())
		if err != nil {
			return err
		}
		if changed {
			fmt.Println(path)
		}
	}
	return nil
}

// alignFile rewrites a single file in place, leaving it untouched when the
// result is identical. It reports whether the file was rewritten.
func alignFile(path string, perm os.FileMode) (bool, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return false, err
	}
	formatted := mdtable.AlignGraphString(string(data))
	if formatted == string(data) {
		return false, nil
	}
	if err := os.WriteFile(path, []byte(formatted), perm); err != nil {
		return false, err
	}
	return true, nil
}
