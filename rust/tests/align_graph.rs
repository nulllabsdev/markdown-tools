//! Align-graph tests over the shared `testdata/graph` fixtures plus focused
//! per-behavior cases, mirroring the Go suite in `go/align_graph_test.go`.

use std::fs;
use std::path::{Path, PathBuf};

use markdown_tools::{align_graph_directory, align_graph_str};

fn graph_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("testdata")
        .join("graph")
}

fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(graph_dir())
        .expect("read testdata/graph dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("input") {
                Some(path.file_stem()?.to_str()?.to_string())
            } else {
                None
            }
        })
        .collect();
    assert!(!names.is_empty(), "no graph fixtures in {:?}", graph_dir());
    names.sort();
    names
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn align_graph_matches_fixtures() {
    let dir = graph_dir();
    for name in fixture_names() {
        let input = read(&dir.join(format!("{name}.input")));
        let want = read(&dir.join(format!("{name}.output")));
        assert_eq!(align_graph_str(&input), want, "fixture `{name}` mismatch");
    }
}

#[test]
fn idempotent() {
    let dir = graph_dir();
    for name in fixture_names() {
        let out = read(&dir.join(format!("{name}.output")));
        assert_eq!(
            align_graph_str(&out),
            out,
            "fixture `{name}` not idempotent"
        );
    }
}

#[test]
fn single_box_normalizes() {
    let input = "```text\n+--------+\n|  Hi    |\n+--------+\n```\n";
    let want = String::from("```text\n")
        + "                        +------------------------------+\n"
        + "                        |              Hi              |\n"
        + "                        +------------------------------+\n"
        + "```\n";
    assert_eq!(align_graph_str(input), want);
}

#[test]
fn non_graph_text_untouched() {
    let input = "```text\njust prose\nno boxes\n```\n";
    assert_eq!(align_graph_str(input), input);
}

#[test]
fn non_text_fence_untouched() {
    let input = "```go\n+------+\n| code |\n+------+\n```\n";
    assert_eq!(align_graph_str(input), input);
}

#[test]
fn unfenced_untouched() {
    let input = "text with | pipes and +--+ but no fence\n";
    assert_eq!(align_graph_str(input), input);
}

#[test]
fn connector_text_preserved() {
    let input = String::from("```text\n")
        + "                        +------------------------------+\n"
        + "                        |              A               |\n"
        + "                        +------------------------------+\n"
        + "   |\n"
        + "          note\n"
        + "   v\n"
        + "                        +------------------------------+\n"
        + "                        |              B               |\n"
        + "                        +------------------------------+\n"
        + "```\n";
    let got = align_graph_str(&input);
    assert!(
        got.contains("                                       |\n"),
        "structural connector not centered to col 39:\n{got}"
    );
    assert!(
        got.contains("          note\n"),
        "annotation indentation not preserved:\n{got}"
    );
}

#[test]
fn preserves_crlf() {
    let input = "```text\r\n+----+\r\n| X  |\r\n+----+\r\n```\r\n";
    let got = align_graph_str(input);
    assert!(
        got.starts_with("```text\r\n"),
        "CRLF opener not preserved: {got:?}"
    );
    assert!(
        got.ends_with("```\r\n"),
        "CRLF closer not preserved: {got:?}"
    );
    assert_eq!(
        got.matches('\n').count(),
        got.matches("\r\n").count(),
        "inconsistent line endings: {got:?}"
    );
}

#[test]
fn align_graph_directory_in_place() {
    let dir = graph_dir();
    let scratch = std::env::temp_dir().join(format!("mdtools-graph-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();

    fs::write(
        scratch.join("doc.md"),
        read(&dir.join("malformed-box.input")),
    )
    .unwrap();
    fs::write(scratch.join("ok.md"), read(&dir.join("single-box.input"))).unwrap();
    let untouched = "```text\n+--+\n|a |\n+--+\n```\n";
    fs::write(scratch.join("skip.txt"), untouched).unwrap();

    let changed = align_graph_directory(&scratch).unwrap();
    assert_eq!(changed, vec![scratch.join("doc.md")]);
    assert_eq!(
        read(&scratch.join("doc.md")),
        read(&dir.join("malformed-box.output"))
    );
    assert_eq!(read(&scratch.join("skip.txt")), untouched);

    fs::remove_dir_all(&scratch).unwrap();
}
