use super::*;

fn describe(blocks: &SyntaxBlocks, row: usize) -> Vec<String> {
    blocks
        .get_line(row)
        .iter()
        .map(|(color, node)| match node {
            TextNode::Range(range) => format!("{color:?}:{range:?}"),
            TextNode::LineOfChars { len, char } => format!("{color:?}:{len}:{char}"),
        })
        .collect()
}

#[test]
fn interrupted_edited_tree_accepts_further_edits_and_matches_a_fresh_parse() {
    let source = (0..1_000)
        .map(|n| format!(":: block{n}\n> line@{n:020x}\n  Hello 🦊 {{value}}.\n"))
        .collect::<String>();
    let mut rope = Rope::from_str(&source);
    let language = EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    );
    let theme = EditorSyntaxTheme::default();
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language), &theme);
    let mut blocks = SyntaxBlocks::default();
    highlighter.parse_visible(&rope, &mut blocks, None, &theme, 0..40);
    let byte = rope.line_to_byte(2);
    let edit = InputEdit {
        start_byte: byte,
        old_end_byte: byte,
        new_end_byte: byte + 1,
        start_position: Point::new(2, 0),
        old_end_position: Point::new(2, 0),
        new_end_position: Point::new(3, 0),
    };
    rope.insert(rope.line_to_char(2), "\n");
    highlighter.edit(&edit);
    assert!(!highlighter.parse_cancellable(&rope, &mut blocks, &theme, 0..40, || true));
    highlighter.reset_parse();
    let byte = rope.line_to_byte(3);
    rope.insert(rope.line_to_char(3), "\n");
    highlighter.edit(&InputEdit {
        start_byte: byte,
        old_end_byte: byte,
        new_end_byte: byte + 1,
        start_position: Point::new(3, 0),
        old_end_position: Point::new(3, 0),
        new_end_position: Point::new(4, 0),
    });
    let mut expected = SyntaxBlocks::default();
    let mut fresh = SyntaxHighlighter::new();
    fresh.set_language(Some(&language), &theme);
    highlighter.parse(&rope, &mut blocks, None, &theme);
    fresh.parse(&rope, &mut expected, None, &theme);
    for row in 0..rope.len_lines() {
        assert_eq!(
            describe(&blocks, row),
            describe(&expected, row),
            "row {row}"
        );
    }
}

mod linewise;
