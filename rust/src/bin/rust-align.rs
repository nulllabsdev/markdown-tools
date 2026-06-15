//! Command `rust-align` aligns the columns of GitHub-style pipe tables in
//! markdown.
//!
//! With no arguments it reads markdown from stdin and writes the formatted
//! result to stdout. Given one or more paths it formats each in place: a
//! directory is walked recursively for `*.md` files, and a file is rewritten
//! only if its content actually changes.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(err) = run(&args) {
        eprintln!("rust-align: {err}");
        process::exit(1);
    }
}

fn run(args: &[String]) -> io::Result<()> {
    if args.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        io::stdout().write_all(markdown_tools::format_str(&input).as_bytes())?;
        return Ok(());
    }

    for arg in args {
        let path = Path::new(arg);
        if path.is_dir() {
            for changed in markdown_tools::format_directory(path)? {
                println!("{}", changed.display());
            }
        } else if format_file(path)? {
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// Rewrites a single file in place, leaving it untouched when the formatted
/// content is identical. Writing to the existing file keeps its permissions.
/// Returns whether the file was rewritten.
fn format_file(path: &Path) -> io::Result<bool> {
    let data = fs::read_to_string(path)?;
    let formatted = markdown_tools::format_str(&data);
    if formatted == data {
        return Ok(false);
    }
    fs::write(path, formatted)?;
    Ok(true)
}
