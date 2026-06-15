//! Wrapper tests over the shared `testdata/wrap` fixtures plus focused
//! per-behavior cases, mirroring the Go suite in `go/wrap_test.go`.

use std::fs;
use std::path::{Path, PathBuf};

use markdown_tools::{wrap_directory, wrap_str};

fn wrap_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("testdata")
        .join("wrap")
}

/// Base names of every `*.input` wrap fixture, sorted for stable ordering.
fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(wrap_dir())
        .expect("read testdata/wrap dir")
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
        "no wrap fixtures found in {:?}",
        wrap_dir()
    );
    names.sort();
    names
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn wrap_matches_fixtures() {
    let dir = wrap_dir();
    for name in fixture_names() {
        let input = read(&dir.join(format!("{name}.input")));
        let want = read(&dir.join(format!("{name}.output")));
        assert_eq!(wrap_str(&input, 80), want, "fixture `{name}` mismatch");
    }
}

#[test]
fn idempotent() {
    let dir = wrap_dir();
    for name in fixture_names() {
        let out = read(&dir.join(format!("{name}.output")));
        assert_eq!(
            wrap_str(&out, 80),
            out,
            "fixture `{name}` is not idempotent"
        );
    }
}

#[test]
fn custom_width() {
    let input = "alpha beta gamma delta epsilon zeta eta theta\n";
    let want = "alpha beta gamma\ndelta epsilon\nzeta eta theta\n";
    assert_eq!(wrap_str(input, 16), want);
}

#[test]
fn long_word_never_split() {
    let input = "see https://example.com/a/very/long/path/that/exceeds/the/width now\n";
    let want = "see\nhttps://example.com/a/very/long/path/that/exceeds/the/width\nnow\n";
    assert_eq!(wrap_str(input, 20), want);
}

#[test]
fn leaves_blocks_untouched() {
    let input = "# A heading that is quite long but must never ever be wrapped by the tool here\n\n\
        - a list item that is also long enough to exceed the configured wrap width easily\n\n\
        > a block quote line that is long enough to exceed the width but stays as written\n\n\
        | col | another column header that is long |\n| --- | --- |\n\n\
        ```\nthis fenced code line is long enough to exceed the width but must be left alone\n```\n";
    assert_eq!(wrap_str(input, 40), input);
}

#[test]
fn preserves_setext_heading() {
    let input =
        "A setext heading whose text is long enough to exceed the chosen wrap width here\n===\n\nbody\n";
    assert_eq!(wrap_str(input, 40), input);
}

#[test]
fn preserves_front_matter() {
    let input =
        "---\ntitle: A very long front matter value that should never be wrapped by the tool\n---\n\nbody\n";
    assert_eq!(wrap_str(input, 40), input);
}

#[test]
fn preserves_mixed_line_endings() {
    let input = "one two three four five\r\nsix seven eight nine ten\n";
    let want = "one two three\r\nfour five six\r\nseven eight\r\nnine ten\n";
    assert_eq!(wrap_str(input, 14), want);
}

#[test]
fn wrap_directory_in_place() {
    let scratch = std::env::temp_dir().join(format!("mdtools-wrap-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let nested = scratch.join("nested");
    fs::create_dir_all(&nested).unwrap();

    let long =
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi\n";
    fs::write(scratch.join("doc.md"), long).unwrap();
    fs::write(nested.join("deep.md"), long).unwrap();
    fs::write(scratch.join("short.md"), "hi there\n").unwrap();
    let untouched = long;
    fs::write(scratch.join("skip.txt"), untouched).unwrap();

    let changed = wrap_directory(&scratch, 80).unwrap();

    let want_changed = vec![scratch.join("doc.md"), nested.join("deep.md")];
    assert_eq!(changed, want_changed, "reported changed files mismatch");
    assert_eq!(
        read(&scratch.join("skip.txt")),
        untouched,
        "non-md file modified"
    );

    fs::remove_dir_all(&scratch).unwrap();
}
