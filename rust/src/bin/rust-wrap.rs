//! Command `rust-wrap` reflows markdown prose paragraphs to a display width.
//!
//! Usage: `rust-wrap [-n WIDTH] [path ...]`
//!
//! With no path arguments it reads markdown from stdin and writes the wrapped
//! result to stdout. Given one or more paths it wraps each in place: a directory
//! is walked recursively for `*.md` files, and a file is rewritten only if its
//! content actually changes. The full path of every changed file is printed, one
//! per line. The default width is 80 columns; `-n` overrides it.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

const DEFAULT_WIDTH: usize = 80;

fn main() {
    if let Err(err) = run(env::args().skip(1)) {
        eprintln!("rust-wrap: {err}");
        process::exit(1);
    }
}

fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut width = DEFAULT_WIDTH;
    let mut paths: Vec<String> = Vec::new();

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if let Some(value) = parse_width_flag(&arg, &mut args)? {
            width = value;
        } else {
            paths.push(arg);
        }
    }

    wrap(width, &paths).map_err(|e| e.to_string())
}

/// Recognizes `-n N`, `-n=N`, and `-nN`, consuming the value argument for the
/// space-separated form. Returns Ok(None) for any other argument.
fn parse_width_flag(
    arg: &str,
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
) -> Result<Option<usize>, String> {
    let raw = if arg == "-n" {
        args.next()
            .ok_or_else(|| "missing value for -n".to_string())?
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

fn wrap(width: usize, paths: &[String]) -> io::Result<()> {
    if paths.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        io::stdout().write_all(markdown_tools::wrap_str(&input, width).as_bytes())?;
        return Ok(());
    }

    for arg in paths {
        let path = Path::new(arg);
        if path.is_dir() {
            for changed in markdown_tools::wrap_directory(path, width)? {
                println!("{}", changed.display());
            }
        } else if wrap_file(path, width)? {
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// Rewrites a single file in place, leaving it untouched when the wrapped content
/// is identical. Returns whether the file was rewritten.
fn wrap_file(path: &Path, width: usize) -> io::Result<bool> {
    let data = fs::read_to_string(path)?;
    let wrapped = markdown_tools::wrap_str(&data, width);
    if wrapped == data {
        return Ok(false);
    }
    fs::write(path, wrapped)?;
    Ok(true)
}
