use super::*;
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, Range, TextDocumentItem,
    VersionedTextDocumentIdentifier,
};

fn edit(start: (u32, u32), end: (u32, u32), text: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: Some(Range::new(
            Position::new(start.0, start.1),
            Position::new(end.0, end.1),
        )),
        range_length: None,
        text: text.into(),
    }
}

#[test]
fn positions_respect_utf16_and_all_line_endings() {
    let text = "a😀b\r\nc\rd\n";
    for (line, character, expected) in [
        (0, 0, Some(0)),
        (0, 1, Some(1)),
        (0, 2, None),
        (0, 3, Some(5)),
        (0, 99, Some(6)),
        (1, 0, Some(8)),
        (1, 99, Some(9)),
        (2, 0, Some(10)),
        (3, 99, Some(12)),
        (4, 0, None),
    ] {
        assert_eq!(byte_offset(text, Position::new(line, character)), expected);
    }
    assert_eq!(byte_offset("", Position::new(0, 99)), Some(0));
}

#[test]
fn sequential_changes_use_the_previous_result_and_validate_lengths() {
    let changes = [edit((0, 1), (0, 3), "x\r\ny"), edit((1, 1), (1, 2), "z")];
    assert_eq!(apply_changes("a😀b", &changes).as_deref(), Some("ax\r\nyz"));
    let mut emoji = edit((0, 1), (0, 3), "");
    emoji.range_length = Some(2);
    assert_eq!(
        apply_changes("a😀b", &[emoji.clone()]).as_deref(),
        Some("ab")
    );
    emoji.range_length = Some(4);
    assert!(apply_changes("a😀b", &[emoji]).is_none());
    assert!(apply_changes("abc", &[edit((0, 2), (0, 1), "")]).is_none());
    assert!(apply_changes("abc", &[]).is_none());
}

#[test]
fn rejected_batch_preserves_text_version_and_next_valid_dependency() {
    let uri: Uri = "file:///sync.recite".parse().unwrap();
    let mut documents = Documents::default();
    assert!(
        documents.accept(&mut Update::Open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(uri.clone(), "recite".into(), 1, "a😀b".into())
        }))
    );
    let change = |version, content_changes| {
        Update::Change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(uri.clone(), version),
            content_changes,
        })
    };
    assert!(!documents.accept(&mut change(
        2,
        vec![edit((0, 0), (0, 1), "c"), edit((0, 2), (0, 2), "bad")]
    )));
    assert_eq!(documents.0[&uri].version, 1);
    assert_eq!(documents.0[&uri].text, "a😀b");
    let mut valid = change(2, vec![edit((0, 3), (0, 4), "z")]);
    assert!(documents.accept(&mut valid));
    let Update::Change(normalized) = valid else {
        panic!("change expected")
    };
    assert_eq!(normalized.content_changes[0].text, "a😀z");
    assert!(normalized.content_changes[0].range.is_none());
    assert!(!documents.accept(&mut change(2, vec![edit((0, 0), (0, 1), "stale")])));
}

#[test]
fn clamped_ranges_and_full_replacements_can_be_combined() {
    assert_eq!(
        apply_changes("abc\r\ndef", &[edit((0, 99), (1, 1), "x")]).as_deref(),
        Some("abcxef")
    );
    assert_eq!(
        apply_changes("abc", &[edit((0, 99), (0, 100), "x")]).as_deref(),
        Some("abcx")
    );
    let full = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "😀\nabc".into(),
    };
    assert_eq!(
        apply_changes("old", &[full.clone(), edit((0, 2), (1, 1), "x")]).as_deref(),
        Some("😀xbc")
    );
    let invalid_full = TextDocumentContentChangeEvent {
        range_length: Some(3),
        ..full
    };
    assert!(apply_changes("old", &[invalid_full]).is_none());
}
