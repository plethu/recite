use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tree_sitter::{InputEdit, LogType, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&recite_writer_grammar::LANGUAGE.into())
        .unwrap();
    parser
}

fn point(source: &str, byte: usize) -> Point {
    let prefix = &source[..byte];
    Point::new(
        prefix.bytes().filter(|byte| *byte == b'\n').count(),
        prefix.rfind('\n').map_or(byte, |start| byte - start - 1),
    )
}

#[derive(Debug, PartialEq)]
struct Node {
    depth: usize,
    kind: String,
    field: Option<String>,
    bytes: Range<usize>,
    start: Point,
    end: Point,
    named: bool,
    missing: bool,
    error: bool,
}

fn nodes(tree: &Tree) -> Vec<Node> {
    let mut result = Vec::new();
    let mut cursor = tree.walk();
    let mut depth = 0;
    loop {
        let node = cursor.node();
        result.push(Node {
            depth,
            kind: node.kind().to_owned(),
            field: cursor.field_name().map(str::to_owned),
            bytes: node.byte_range(),
            start: node.start_position(),
            end: node.end_position(),
            named: node.is_named(),
            missing: node.is_missing(),
            error: node.is_error(),
        });
        if cursor.goto_first_child() {
            depth += 1;
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return result;
            }
            depth -= 1;
        }
    }
}

fn captures(tree: &Tree, source: &str) -> Vec<(String, Range<usize>)> {
    let query = Query::new(
        &recite_writer_grammar::LANGUAGE.into(),
        recite_writer_grammar::HIGHLIGHTS,
    )
    .unwrap();
    let mut cursor = QueryCursor::new();
    let mut stream = cursor.captures(&query, tree.root_node(), source.as_bytes());
    let mut result = Vec::new();
    while let Some((matched, index)) = stream.next() {
        let capture = matched.captures[*index];
        result.push((
            query.capture_names()[capture.index as usize].to_owned(),
            capture.node.byte_range(),
        ));
    }
    result
}

fn edit(
    parser: &mut Parser,
    tree: &mut Tree,
    source: &mut String,
    range: Range<usize>,
    replacement: &str,
) {
    let start_position = point(source, range.start);
    let old_end_position = point(source, range.end);
    source.replace_range(range.clone(), replacement);
    let new_end_byte = range.start + replacement.len();
    tree.edit(&InputEdit {
        start_byte: range.start,
        old_end_byte: range.end,
        new_end_byte,
        start_position,
        old_end_position,
        new_end_position: point(source, new_end_byte),
    });
    *tree = parser.parse(source.as_bytes(), Some(tree)).unwrap();
}

fn check_cold(tree: &Tree, source: &str) {
    let cold = parser().parse(source.as_bytes(), None).unwrap();
    assert_eq!(nodes(tree), nodes(&cold), "source: {source:?}");
    assert_eq!(captures(tree, source), captures(&cold, source));
}

#[test]
fn boundary_edits_match_cold_trees_fields_spans_and_ordered_captures() {
    let fixtures = [
        include_str!("../../../../../fixtures/recite/valid/language_pressure.recite"),
        include_str!("../../../../../fixtures/recite/invalid/parser_recoverable_malformed.recite"),
        ":: root default\r\n> first\r\n  🦊 prose.\r\n  \r\n  :: next\r\n> last\r\n  Last prose.",
        ":: root\n! immediate call(\"\n\n> later\n  Later text.\n-> END",
        ":: root\n> first\n  Two slashes \\\\{name}\n> next\n  Later text.\n-> END\n",
    ];
    for phase in 0..8 {
        for fixture in fixtures {
            let original = format!("{}{fixture}", "\n".repeat(phase));
            let middle = original
                .char_indices()
                .nth(original.chars().count() / 2)
                .map_or(0, |(byte, _)| byte);
            for anchor in [0, middle, original.len()] {
                let mut source = original.clone();
                let mut parser = parser();
                let mut tree = parser.parse(&source, None).unwrap();
                for inserted in ["x", "\n", "\r\n", "\n\n", "\"", "[", "🦊"] {
                    edit(
                        &mut parser,
                        &mut tree,
                        &mut source,
                        anchor..anchor,
                        inserted,
                    );
                    check_cold(&tree, &source);
                    edit(
                        &mut parser,
                        &mut tree,
                        &mut source,
                        anchor..anchor + inserted.len(),
                        "",
                    );
                    check_cold(&tree, &source);
                    assert_eq!(source, original);
                }
            }
        }
    }
}

#[test]
fn character_edit_repetition_work_is_bounded_on_sources_without_blank_separators() {
    for lines in [100, 1_000, 10_000] {
        let mut source = format!(
            ":: scale default\n{}# tail",
            "  Plain 🦊 prose.\n".repeat(lines)
        );
        let mut parser = parser();
        let mut tree = parser.parse(&source, None).unwrap();
        assert!(!tree.root_node().has_error());
        assert_eq!(tree.root_node().named_child_count(), lines + 2);
        let work = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&work);
        parser.set_logger(Some(Box::new(move |kind, _| {
            if matches!(kind, LogType::Parse) {
                observed.fetch_add(1, Ordering::Relaxed);
            }
        })));
        let end = source.len();
        edit(&mut parser, &mut tree, &mut source, end..end, "x");
        let operations = work.load(Ordering::Relaxed);
        assert!(operations > 0, "parser work logging must be active");
        // The 8-line grouping measured below one parse-log event per line;
        // the former flat repeat emits about five. Retain headroom for the
        // changed comment and bounded suffix, without a wall-clock assertion.
        assert!(
            operations <= lines + 128,
            "lines={lines}, work={operations}"
        );
        assert!(!tree.root_node().has_error());
        assert!(source.ends_with("# tailx"));
        check_cold(&tree, &source);
    }
}
