use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

const DEFAULT_WIDTH: usize = 80;
const VERSION_BUILD: &str = env!("MARKDOWN_TOOLS_VERSION");

fn main() {
    if let Err(err) = run(env::args().skip(1), &mut io::stdin(), &mut io::stdout()) {
        eprintln!("rustmd: {err}");
        process::exit(1);
    }
}

fn run(
    args: impl Iterator<Item = String>,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<(), String> {
    let args: Vec<String> = args.collect();
    if args.len() == 1 && args[0] == "-v" {
        write_version(stdout).map_err(|e| e.to_string())?;
        writeln!(stdout).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let Some((subcommand, rest)) = args.split_first() else {
        return Err(usage());
    };

    let mut body = Vec::new();
    let result = match subcommand.as_str() {
        "align" => run_align(rest, stdin, &mut body).map_err(|e| e.to_string()),
        "wrap" => {
            let (width, paths) = parse_width_args(rest)?;
            run_wrap(width, &paths, stdin, &mut body).map_err(|e| e.to_string())
        }
        "graph" => run_graph(rest, stdin, &mut body).map_err(|e| e.to_string()),
        "all" => {
            let (width, paths) = parse_width_args(rest)?;
            run_all(width, &paths, stdin, &mut body).map_err(|e| e.to_string())
        }
        _ => Err(usage()),
    };

    result?;
    write_version(stdout).map_err(|e| e.to_string())?;
    stdout.write_all(&body).map_err(|e| e.to_string())?;
    writeln!(stdout).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_version(stdout: &mut impl Write) -> io::Result<()> {
    writeln!(stdout, "build {VERSION_BUILD}")
}

fn usage() -> String {
    "usage: rustmd <align|wrap|graph|all> [flags] [paths...]\n  wrap/all flags: -n N (default 80)"
        .to_string()
}

fn run_align(paths: &[String], stdin: &mut impl Read, stdout: &mut impl Write) -> io::Result<()> {
    run_simple(paths, stdin, stdout, markdown_tools::format_str, |path| {
        markdown_tools::format_directory(path)
    })
}

fn run_wrap(
    width: usize,
    paths: &[String],
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> io::Result<()> {
    run_simple(
        paths,
        stdin,
        stdout,
        |s| markdown_tools::wrap_str(s, width),
        |path| markdown_tools::wrap_directory(path, width),
    )
}

fn run_graph(paths: &[String], stdin: &mut impl Read, stdout: &mut impl Write) -> io::Result<()> {
    run_simple(
        paths,
        stdin,
        stdout,
        markdown_tools::align_graph_str,
        |path| markdown_tools::align_graph_directory(path),
    )
}

fn run_all(
    width: usize,
    paths: &[String],
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> io::Result<()> {
    let transform = |s: &str| {
        markdown_tools::align_graph_str(&markdown_tools::wrap_str(
            &markdown_tools::format_str(s),
            width,
        ))
    };

    if paths.is_empty() {
        let mut input = String::new();
        stdin.read_to_string(&mut input)?;
        stdout.write_all(transform(&input).as_bytes())?;
        return Ok(());
    }

    let mut changed = BTreeSet::new();
    for arg in paths {
        let path = Path::new(arg);
        if path.is_dir() {
            for runner in [
                DirectoryRunner::Align,
                DirectoryRunner::Wrap(width),
                DirectoryRunner::Graph,
            ] {
                for file in runner.run(path)? {
                    changed.insert(file);
                }
            }
        } else if rewrite_file(path, &transform)? {
            changed.insert(path.to_path_buf());
        }
    }

    for path in changed {
        writeln!(stdout, "{}", path.display())?;
    }
    Ok(())
}

fn run_simple(
    paths: &[String],
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    transform: impl Fn(&str) -> String,
    walk_dir: impl Fn(&Path) -> io::Result<Vec<std::path::PathBuf>>,
) -> io::Result<()> {
    if paths.is_empty() {
        let mut input = String::new();
        stdin.read_to_string(&mut input)?;
        stdout.write_all(transform(&input).as_bytes())?;
        return Ok(());
    }

    for arg in paths {
        let path = Path::new(arg);
        if path.is_dir() {
            for changed in walk_dir(path)? {
                writeln!(stdout, "{}", changed.display())?;
            }
        } else if rewrite_file(path, &transform)? {
            writeln!(stdout, "{}", path.display())?;
        }
    }
    Ok(())
}

fn parse_width_args(args: &[String]) -> Result<(usize, Vec<String>), String> {
    let mut width = DEFAULT_WIDTH;
    let mut paths = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if let Some(value) = parse_width_flag(arg, &mut iter)? {
            width = value;
        } else {
            paths.push(arg.clone());
        }
    }
    Ok((width, paths))
}

fn parse_width_flag<'a>(
    arg: &str,
    args: &mut std::iter::Peekable<impl Iterator<Item = &'a String>>,
) -> Result<Option<usize>, String> {
    let raw = if arg == "-n" {
        args.next()
            .ok_or_else(|| "missing value for -n".to_string())?
            .to_string()
    } else if let Some(rest) = arg.strip_prefix("-n=").or_else(|| arg.strip_prefix("-n")) {
        rest.to_string()
    } else {
        return Ok(None);
    };

    let value: usize = raw.parse().map_err(|_| format!("invalid width {raw:?}"))?;
    if value < 1 {
        return Err("width must be at least 1".to_string());
    }
    Ok(Some(value))
}

fn rewrite_file(path: &Path, transform: &impl Fn(&str) -> String) -> io::Result<bool> {
    let data = fs::read_to_string(path)?;
    let out = transform(&data);
    if out == data {
        return Ok(false);
    }
    fs::write(path, out)?;
    Ok(true)
}

enum DirectoryRunner {
    Align,
    Wrap(usize),
    Graph,
}

impl DirectoryRunner {
    fn run(&self, path: &Path) -> io::Result<Vec<std::path::PathBuf>> {
        match self {
            Self::Align => markdown_tools::format_directory(path),
            Self::Wrap(width) => markdown_tools::wrap_directory(path, *width),
            Self::Graph => markdown_tools::align_graph_directory(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn unknown_subcommand_returns_usage() {
        let err = run(
            ["wat".to_string()].into_iter(),
            &mut Cursor::new(""),
            &mut Vec::new(),
        )
        .expect_err("expected error");
        assert!(err.contains("usage: rustmd"));
    }

    #[test]
    fn version_flag_prints_version() {
        let mut stdout = Vec::new();
        run(
            ["-v".to_string()].into_iter(),
            &mut Cursor::new(""),
            &mut stdout,
        )
        .unwrap();
        let got = String::from_utf8(stdout).unwrap();
        assert_eq!(got, format!("build {VERSION_BUILD}\n\n"));
    }

    #[test]
    fn all_composes_stdin() {
        let input = "| a | bb |\n|---|---|\n| 1 | 2 |\n\nalpha beta gamma delta epsilon\n";
        let mut stdout = Vec::new();
        run(
            ["all".to_string(), "-n".to_string(), "12".to_string()].into_iter(),
            &mut Cursor::new(input),
            &mut stdout,
        )
        .unwrap();
        let got = String::from_utf8(stdout).unwrap();
        let want = format!(
            "build {VERSION_BUILD}\n{}",
            "| a   | bb  |\n| --- | --- |\n| 1   | 2   |\n\nalpha beta\ngamma delta\nepsilon\n\n",
        );
        assert_eq!(got, want);
    }

    #[test]
    fn wrap_preserves_trailing_markdown_link() {
        let input =
            "and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n";
        let mut stdout = Vec::new();
        run(
            ["wrap".to_string(), "-n".to_string(), "40".to_string()].into_iter(),
            &mut Cursor::new(input),
            &mut stdout,
        )
        .unwrap();
        let got = String::from_utf8(stdout).unwrap();
        let want = format!("build {VERSION_BUILD}\n{input}\n");
        assert_eq!(got, want);
    }

    #[test]
    fn wrap_preserves_list_continuation_indent() {
        let input = "- `FormatDirectory(root string) ([]string, error)` — `filepath.WalkDir` over\n  `root`, formatting every `*.md` file in place (read → `FormatString` → write\n  back only when the content changes, preserving file mode) and returning the\n  changed paths in walk order.\n";
        let mut stdout = Vec::new();
        run(
            ["wrap".to_string(), "-n".to_string(), "80".to_string()].into_iter(),
            &mut Cursor::new(input),
            &mut stdout,
        )
        .unwrap();
        let got = String::from_utf8(stdout).unwrap();
        let want = format!("build {VERSION_BUILD}\n{input}\n");
        assert_eq!(got, want);
    }
}
