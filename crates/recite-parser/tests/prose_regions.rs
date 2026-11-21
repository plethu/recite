#![cfg(test)]

use std::fmt::Write as _;
use std::path::PathBuf;

use recite_parser::{ProsePieceKind, ProseRegionKind, prose_regions};

fn fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// Render every region as `line:column` source excerpts plus the checked text,
/// so a snapshot shows exactly which bytes count as prose.
fn render(source: &str) -> String {
    let mut output = String::new();
    for region in prose_regions(source) {
        let _ = writeln!(
            output,
            "{} id={} speaker={} key={}",
            region.kind().as_str(),
            region.stable_id().unwrap_or("-"),
            region.speaker().unwrap_or("-"),
            region.metadata_key().unwrap_or("-"),
        );
        for line in region.lines() {
            let bytes = line.bytes();
            let (number, column) = line_column(source, bytes.start);
            let _ = writeln!(
                output,
                "  {number}:{column}{} {:?}",
                if line.paragraph_start() { " ¶" } else { "" },
                &source[bytes],
            );
        }
        let text = region.text(source);
        let _ = writeln!(output, "  checked {:?}", text.as_str());
        for piece in text.pieces() {
            if piece.kind() != ProsePieceKind::Prose {
                let _ = writeln!(
                    output,
                    "    {:?} {:?} <- {:?}",
                    piece.kind(),
                    &text.as_str()[piece.text_range()],
                    &source[piece.source_range()],
                );
            }
        }
    }
    output
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |line| line.chars().count())
        + 1;
    (line, column)
}

fn assert_snapshot(name: &str, actual: &str) {
    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    insta::with_settings!({ snapshot_path => snapshot_path }, {
        insta::assert_snapshot!(name, actual);
    });
}

#[test]
fn harbour_fixture_regions_cover_prose_and_exclude_syntax() {
    let source = fixture("fixtures/recite/valid/prose/harbour.recite");
    assert_snapshot("prose_regions_harbour", &render(&source));
}

#[test]
fn incomplete_fixture_still_reports_recognised_prose() {
    let source = fixture("fixtures/recite/invalid/prose/incomplete.recite");
    assert_snapshot("prose_regions_incomplete", &render(&source));
}

#[test]
fn regions_never_include_header_comment_condition_or_divert_bytes() {
    let source = fixture("fixtures/recite/valid/prose/harbour.recite");
    for region in prose_regions(&source) {
        for line in region.lines() {
            let text = &source[line.bytes()];
            for syntax in ["@", "->", ":if", ":else", "speaker=", "bind=", "::"] {
                assert!(!text.contains(syntax), "{syntax} leaked into {text:?}");
            }
            assert!(!text.starts_with('#') && !text.starts_with('>') && !text.starts_with('?'));
        }
    }
}

#[test]
fn crlf_and_tab_indented_bodies_map_to_exact_bytes() {
    let source =
        ":: start default\r\n> @a1\r\n\tFirst line\r\n\tsecond line\r\n\r\n\tNew paragraph.\r\n";
    let regions = prose_regions(source);
    assert_eq!(regions.len(), 1);
    let lines = regions[0]
        .lines()
        .iter()
        .map(|line| (&source[line.bytes()], line.paragraph_start()))
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        [
            ("First line", true),
            ("second line", false),
            ("New paragraph.", true)
        ]
    );
    let text = regions[0].text(source);
    assert_eq!(text.as_str(), "First line\nsecond line\n\nNew paragraph.");
}

#[test]
fn source_map_is_exact_inside_prose_and_covering_across_syntax() {
    let source = ":: s default\n> bind=(n:string=$n) @a1\n  Hi [b]{n}[/b], wellcome.\n";
    let region = &prose_regions(source)[0];
    let text = region.text(source);
    assert_eq!(text.as_str(), "Hi {n}, wellcome.");
    let word = text.as_str().find("wellcome").expect("word");
    let exact = text
        .exact_source_range(word..word + "wellcome".len())
        .expect("exact range inside prose");
    assert_eq!(&source[exact], "wellcome");
    let placeholder = text.as_str().find("{n}").expect("placeholder");
    assert!(text.overlaps_placeholder(placeholder..placeholder + 3));
    assert!(text.exact_source_range(0..placeholder + 3).is_none());
    let covering = text.covering_source_range(0..placeholder + 3);
    assert_eq!(&source[covering], "Hi [b]{n}");
}

#[test]
fn markup_shapes_are_masked_but_bracketed_prose_is_checked() {
    let source =
        ":: s default\n> @a1\n  [pause=2][color value=red]Red[/color] [The door opens.] [/ b]\n";
    let text = prose_regions(source)[0].text(source);
    assert_eq!(text.as_str(), "Red [The door opens.] [/ b]");
    let markup = text
        .pieces()
        .iter()
        .filter(|piece| piece.kind() == ProsePieceKind::Markup)
        .count();
    assert_eq!(markup, 3);
}

#[test]
fn metadata_regions_cover_quoted_values_only() {
    let source = ":: s default caption=\"Block title\"\n> speaker=rhea note=symbol caption=\"Say \\\"hi\\\"\" @a1\n  Body.\n";
    let regions = prose_regions(source);
    let metadata = regions
        .iter()
        .filter(|region| region.kind() == ProseRegionKind::Metadata)
        .map(|region| {
            (
                region.metadata_key().unwrap_or_default(),
                region.stable_id().unwrap_or_default(),
                region.text(source).as_str().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        metadata,
        [
            ("caption", "s", "Block title".to_owned()),
            ("caption", "a1", "Say \"hi\"".to_owned()),
        ]
    );
}
