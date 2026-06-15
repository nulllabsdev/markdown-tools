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
fn preserves_mixed_line_endings() {
    let input = "intro\r\n| A | B |\n| --- | --- |\r\n| x | yy |\nend";
    let want = "intro\r\n| A   | B   |\n| --- | --- |\r\n| x   | yy  |\nend";
    assert_eq!(format_str(input), want);
}

#[test]
fn code_fence_close_requires_only_fence_markers() {
    let input =
        "```markdown\n```not a closer\n| not | touched |\n|-|-|\n```\n| yes | formatted |\n| --- | --- |\n";
    let want =
        "```markdown\n```not a closer\n| not | touched |\n|-|-|\n```\n| yes | formatted |\n| --- | --------- |\n";
    assert_eq!(format_str(input), want);
}

#[test]
fn backtick_fence_info_string_rejects_backtick() {
    // A backtick fence whose info string contains a backtick is not a fence, so
    // the table after it must still be formatted.
    let input = "```js`x\n| a | b |\n|-|-|\n| 1 | 2 |\n";
    let want = "```js`x\n| a   | b   |\n| --- | --- |\n| 1   | 2   |\n";
    assert_eq!(format_str(input), want);

    // A tilde fence's info string may contain backticks, so it still opens a
    // fence and the table inside is left untouched.
    let tilde = "~~~js`x\n| a | b |\n|-|-|\n~~~\n";
    assert_eq!(format_str(tilde), tilde);
}

#[test]
fn fence_indentation() {
    // Four-space indentation is indented-code content, not a fence, so the line
    // does not open a fence and the table after it is formatted.
    let deep = "    ```\n| a | b |\n|-|-|\n| 1 | 2 |\n";
    let want_deep = "    ```\n| a   | b   |\n| --- | --- |\n| 1   | 2   |\n";
    assert_eq!(format_str(deep), want_deep);

    // Up to three spaces still opens (and closes) a fence, so the table between
    // the markers is left untouched.
    let shallow = "   ```\n| a | b |\n|-|-|\n   ```\n";
    assert_eq!(format_str(shallow), shallow);
}

#[test]
fn format_directory_accepts_markdown_file_root() {
    let dir = std::env::temp_dir().join(format!(
        "mdtools-rust-file-root-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let path = dir.join("single.md");
    fs::write(&path, read(&testdata_dir().join("emoji.input"))).unwrap();

    let changed = format_directory(&path).unwrap();
    assert_eq!(changed, vec![path.clone()]);
    assert_eq!(read(&path), read(&testdata_dir().join("emoji.output")));

    fs::remove_dir_all(&dir).unwrap();
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

    let md_fixtures = ["three-alignments", "cjk", "ascii-flowchart"];
    for f in md_fixtures {
        let input = read(&dir.join(format!("{f}.input")));
        fs::write(scratch.join(format!("{f}.md")), input).unwrap();
    }
    // Nested *.md to confirm recursion.
    fs::write(nested.join("emoji.md"), read(&dir.join("emoji.input"))).unwrap();

    // A non-markdown file that must be left exactly as-is.
    let untouched = "| a | b |\n| - | - |\n";
    fs::write(scratch.join("skip.txt"), untouched).unwrap();

    let changed = format_directory(&scratch).unwrap();

    // Every *.md fixture differs from its formatted form; the .txt file must not
    // be reported. Order matches the walker's sorted, depth-first traversal.
    let want_changed = vec![
        scratch.join("ascii-flowchart.md"),
        scratch.join("cjk.md"),
        nested.join("emoji.md"),
        scratch.join("three-alignments.md"),
    ];
    assert_eq!(changed, want_changed, "reported changed files mismatch");

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
