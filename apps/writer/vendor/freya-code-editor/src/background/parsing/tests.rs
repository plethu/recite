use super::*;
use tree_sitter::{InputEdit, Point};

fn request(
    base: Option<super::super::Stamp>,
    rope: &Rope,
    edits: Option<Vec<InputEdit>>,
) -> Request {
    Request {
        base,
        stamp: Arc::new(()),
        rope: rope.clone(),
        edits,
        rows: 0..40,
        language: Some(EditorLanguage::new(
            recite_writer_grammar::LANGUAGE,
            recite_writer_grammar::HIGHLIGHTS,
        )),
        theme: EditorSyntaxTheme::default(),
    }
}
fn describe(blocks: &SyntaxBlocks, row: usize) -> Vec<String> {
    blocks
        .get_line(row)
        .iter()
        .map(|(color, node)| match node {
            crate::syntax::TextNode::Range(range) => format!("{color:?}:{range:?}"),
            crate::syntax::TextNode::LineOfChars { len, char } => format!("{color:?}:{len}:{char}"),
        })
        .collect()
}
#[test]
fn viewport_after_interrupted_edit_reparses_before_querying_and_keeps_undo_coordinates() {
    let source = (0..1_000)
        .map(|n| format!(":: block{n}\n> line@{n:020x}\n  Hello 🦊 {{value}}.\n"))
        .collect::<String>();
    let mut rope = Rope::from_str(&source);
    let mailbox = Mailbox::default();
    let mut parsing = Parsing::default();
    let first = request(None, &rope, None);
    let first_stamp = first.stamp.clone();
    assert!(parsing.highlight(first, &mailbox).is_some());
    let byte = rope.line_to_byte(2);
    rope.insert(rope.line_to_char(2), "\n# ");
    let edit = InputEdit {
        start_byte: byte,
        old_end_byte: byte,
        new_end_byte: byte + 3,
        start_position: Point::new(2, 0),
        old_end_position: Point::new(2, 0),
        new_end_position: Point::new(3, 2),
    };
    let changed = request(Some(first_stamp), &rope, Some(vec![edit]));
    let changed_stamp = changed.stamp.clone();
    mailbox.superseded.store(true, Ordering::Relaxed);
    assert!(parsing.highlight(changed, &mailbox).is_none());
    assert!(!parsing.parsed);
    mailbox.superseded.store(false, Ordering::Relaxed);
    let viewport = request(Some(changed_stamp), &rope, Some(Vec::new()));
    let viewport_stamp = viewport.stamp.clone();
    let output = parsing.highlight(viewport, &mailbox).unwrap();
    let expected = Parsing::default()
        .highlight(request(None, &rope, None), &mailbox)
        .unwrap();
    for row in 0..40 {
        assert_eq!(
            describe(&output.blocks, row),
            describe(&expected.blocks, row),
            "row {row}"
        );
    }
    let char_offset = rope.byte_to_char(byte);
    rope.remove(char_offset..char_offset + 3);
    let undo = InputEdit {
        start_byte: byte,
        old_end_byte: byte + 3,
        new_end_byte: byte,
        start_position: Point::new(2, 0),
        old_end_position: Point::new(3, 2),
        new_end_position: Point::new(2, 0),
    };
    let output = parsing
        .highlight(
            request(Some(viewport_stamp), &rope, Some(vec![undo])),
            &mailbox,
        )
        .unwrap();
    let expected = Parsing::default()
        .highlight(request(None, &rope, None), &mailbox)
        .unwrap();
    assert_eq!(String::from(&rope), source);
    for row in 0..40 {
        assert_eq!(
            describe(&output.blocks, row),
            describe(&expected.blocks, row),
            "row {row}"
        );
    }
}
