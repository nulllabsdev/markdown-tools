//! Command `rust-align-graph` re-renders ASCII flowcharts inside fenced ` ```text `
//! blocks to a canonical centered form.
//!
//! With no arguments it reads markdown from stdin and writes the result to
//! stdout. Given one or more paths it rewrites each in place: a directory is
//! walked recursively for `*.md` files, and a file is rewritten only if its
//! content actually changes. The full path of every changed file is printed, one
//! per line.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(err) = run(&args) {
        eprintln!("rust-align-graph: {err}");
        process::exit(1);
    }
}

fn run(args: &[String]) -> io::Result<()> {
    if args.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        io::stdout().write_all(markdown_tools::align_graph_str(&input).as_bytes())?;
        return Ok(());
    }

    for arg in args {
        let path = Path::new(arg);
        if path.is_dir() {
            for changed in markdown_tools::align_graph_directory(path)? {
                println!("{}", changed.display());
            }
        } else if align_file(path)? {
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// Rewrites a single file in place, leaving it untouched when the result is
/// identical. Returns whether the file was rewritten.
fn align_file(path: &Path) -> io::Result<bool> {
    let data = fs::read_to_string(path)?;
    let formatted = markdown_tools::align_graph_str(&data);
    if formatted == data {
        return Ok(false);
    }
    fs::write(path, formatted)?;
    Ok(true)
}
