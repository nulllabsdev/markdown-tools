//! Integration tests that drive the formatter from the shared `testdata/`
//! fixtures, so the Rust and Go implementations are verified identically.

use std::fs;
use std::path::{Path, PathBuf};

use markdown_tools::{format_directory, format_str};

fn testdata_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("testdata")
}

/// Base names of every `*.input` fixture, sorted for stable test ordering.
fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(testdata_dir())
        .expect("read testdata dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("input") {
                Some(path.file_stem()?.to_str()?.to_string())
            } else {
                None
            }
        })
        .collect();
    assert!(
        !names.is_empty(),
        "no fixtures found in {:?}",
        testdata_dir()
    );
    names.sort();
    names
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn format_matches_fixtures() {
    let dir = testdata_dir();
    for name in fixture_names() {
        let input = read(&dir.join(format!("{name}.input")));
        let want = read(&dir.join(format!("{name}.output")));
        assert_eq!(format_str(&input), want, "fixture `{name}` mismatch");
    }
}

#[test]
fn idempotent() {
    let dir = testdata_dir();
    for name in fixture_names() {
        let out = read(&dir.join(format!("{name}.output")));
        assert_eq!(format_str(&out), out, "fixture `{name}` is not idempotent");
    }
}

#[test]
fn format_directory_in_place() {
    let dir = testdata_dir();

    // Unique scratch directory under the system temp dir (no extra deps).
    let scratch = std::env::temp_dir().join(format!("mdtools-rust-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let nested = scratch.join("nested");
    fs::create_dir_all(&nested).unwrap();

    let md_fixtures = ["three-alignments", "cjk"];
    for f in md_fixtures {
        let input = read(&dir.join(format!("{f}.input")));
        fs::write(scratch.join(format!("{f}.md")), input).unwrap();
    }
    // Nested *.md to confirm recursion.
    fs::write(nested.join("emoji.md"), read(&dir.join("emoji.input"))).unwrap();

    // A non-markdown file that must be left exactly as-is.
    let untouched = "| a | b |\n| - | - |\n";
    fs::write(scratch.join("skip.txt"), untouched).unwrap();

    format_directory(&scratch).unwrap();

    for f in md_fixtures {
        let want = read(&dir.join(format!("{f}.output")));
        let got = read(&scratch.join(format!("{f}.md")));
        assert_eq!(got, want, "{f}.md not formatted in place");
    }
    let want_emoji = read(&dir.join("emoji.output"));
    assert_eq!(
        read(&nested.join("emoji.md")),
        want_emoji,
        "nested emoji.md not formatted"
    );
    assert_eq!(
        read(&scratch.join("skip.txt")),
        untouched,
        "non-md file was modified"
    );

    fs::remove_dir_all(&scratch).unwrap();
}
