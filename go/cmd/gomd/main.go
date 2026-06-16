package main

import (
	"flag"
	"fmt"
	"io"
	"os"
	"slices"

	"github.com/nulllabsdev/markdown-tools/go"
)

const defaultWidth = 80

func main() {
	os.Exit(run(os.Args[1:], os.Stdin, os.Stdout, os.Stderr))
}

func run(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		printUsage(stderr)
		return 1
	}

	var err error
	switch args[0] {
	case "align":
		err = runAlign(args[1:], stdin, stdout)
	case "wrap":
		err = runWrap(args[1:], stdin, stdout)
	case "graph":
		err = runGraph(args[1:], stdin, stdout)
	case "all":
		err = runAll(args[1:], stdin, stdout)
	default:
		printUsage(stderr)
		return 1
	}
	if err != nil {
		fmt.Fprintln(stderr, "gomd:", err)
		return 1
	}
	return 0
}

func printUsage(w io.Writer) {
	fmt.Fprintln(w, "usage: gomd <align|wrap|graph|all> [flags] [paths...]")
	fmt.Fprintln(w, "  wrap/all flags: -n N (default 80)")
}

func runAlign(args []string, stdin io.Reader, stdout io.Writer) error {
	return runSimple(args, stdin, stdout, mdtable.FormatString, mdtable.FormatDirectory)
}

func runWrap(args []string, stdin io.Reader, stdout io.Writer) error {
	width, paths, err := parseWidthArgs("wrap", args)
	if err != nil {
		return err
	}
	return runSimple(paths, stdin, stdout,
		func(s string) string { return mdtable.WrapString(s, width) },
		func(path string) ([]string, error) { return mdtable.WrapDirectory(path, width) },
	)
}

func runGraph(args []string, stdin io.Reader, stdout io.Writer) error {
	return runSimple(args, stdin, stdout, mdtable.AlignGraphString, mdtable.AlignGraphDirectory)
}

func runAll(args []string, stdin io.Reader, stdout io.Writer) error {
	width, paths, err := parseWidthArgs("all", args)
	if err != nil {
		return err
	}

	transform := func(s string) string {
		return mdtable.AlignGraphString(mdtable.WrapString(mdtable.FormatString(s), width))
	}

	if len(paths) == 0 {
		data, err := io.ReadAll(stdin)
		if err != nil {
			return err
		}
		_, err = io.WriteString(stdout, transform(string(data)))
		return err
	}

	changedSet := map[string]struct{}{}
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			return err
		}
		if info.IsDir() {
			for _, fn := range []func(string) ([]string, error){
				mdtable.FormatDirectory,
				func(root string) ([]string, error) { return mdtable.WrapDirectory(root, width) },
				mdtable.AlignGraphDirectory,
			} {
				changed, err := fn(path)
				if err != nil {
					return err
				}
				for _, p := range changed {
					changedSet[p] = struct{}{}
				}
			}
			continue
		}
		changed, err := rewriteFile(path, info.Mode().Perm(), transform)
		if err != nil {
			return err
		}
		if changed {
			changedSet[path] = struct{}{}
		}
	}

	pathsOut := make([]string, 0, len(changedSet))
	for path := range changedSet {
		pathsOut = append(pathsOut, path)
	}
	slices.Sort(pathsOut)
	for _, path := range pathsOut {
		fmt.Fprintln(stdout, path)
	}
	return nil
}

func runSimple(
	paths []string,
	stdin io.Reader,
	stdout io.Writer,
	transform func(string) string,
	walkDir func(string) ([]string, error),
) error {
	if len(paths) == 0 {
		data, err := io.ReadAll(stdin)
		if err != nil {
			return err
		}
		_, err = io.WriteString(stdout, transform(string(data)))
		return err
	}

	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			return err
		}
		if info.IsDir() {
			changed, err := walkDir(path)
			if err != nil {
				return err
			}
			for _, p := range changed {
				fmt.Fprintln(stdout, p)
			}
			continue
		}
		changed, err := rewriteFile(path, info.Mode().Perm(), transform)
		if err != nil {
			return err
		}
		if changed {
			fmt.Fprintln(stdout, path)
		}
	}
	return nil
}

func parseWidthArgs(name string, args []string) (int, []string, error) {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	width := fs.Int("n", defaultWidth, "wrap width in display columns")
	if err := fs.Parse(args); err != nil {
		return 0, nil, err
	}
	if *width < 1 {
		return 0, nil, fmt.Errorf("width must be at least 1, got %d", *width)
	}
	return *width, fs.Args(), nil
}

func rewriteFile(path string, perm os.FileMode, transform func(string) string) (bool, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return false, err
	}
	out := transform(string(data))
	if out == string(data) {
		return false, nil
	}
	if err := os.WriteFile(path, []byte(out), perm); err != nil {
		return false, err
	}
	return true, nil
}
