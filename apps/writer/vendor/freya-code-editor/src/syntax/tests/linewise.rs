use super::*;
fn language() -> EditorLanguage {
    EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    )
    .with_line_isolated_highlighting()
}
fn replace(rope: &mut Rope, start: usize, end: usize, text: &str) -> InputEdit {
    // Tree-sitter positions count LF bytes even when Rope recognizes bare CR.
    let point = |rope: &Rope, end| {
        let mut point = Point::new(0, 0);
        for byte in rope.byte_slice(..end).bytes() {
            if byte == b'\n' {
                point.row += 1;
                point.column = 0;
            } else {
                point.column += 1;
            }
        }
        point
    };
    let start_byte = rope.char_to_byte(start);
    let old_end_byte = rope.char_to_byte(end);
    let start_position = point(rope, start_byte);
    let old_end_position = point(rope, old_end_byte);
    rope.remove(start..end);
    rope.insert(start, text);
    let new_end_byte = start_byte + text.len();
    InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position,
        old_end_position,
        new_end_position: point(rope, new_end_byte),
    }
}
fn parity(highlighter: &mut SyntaxHighlighter, rope: &Rope, rows: Range<usize>) {
    let theme = EditorSyntaxTheme::default();
    let mut actual = SyntaxBlocks::default();
    assert!(highlighter.parse_cancellable(rope, &mut actual, &theme, rows.clone(), || false));
    let mut expected = SyntaxBlocks::default();
    let mut fresh = SyntaxHighlighter::new();
    let language = EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    );
    fresh.set_language(Some(&language), &theme);
    fresh.parse_visible(rope, &mut expected, None, &theme, rows.clone());
    for row in rows {
        assert_eq!(
            describe(&actual, row),
            describe(&expected, row),
            "row {row}, {rope}"
        );
    }
}
#[test]
fn clean_line_edits_match_full_projection_across_grouping_phases_and_split_join_boundaries() {
    let theme = EditorSyntaxTheme::default();
    let mut accepted = 0;
    for phase in 0..8 {
        let source = format!(
            "{}:: start default\n> line@00000000000000000000\n  Café 🦊 {{value}}.\n? choice\n  Choice.\n  -> END\n# comment\n:: next\n> next\n  Next.\n-> END\n",
            "\n".repeat(phase)
        );
        let chars = source.chars().count();
        for offset in 0..chars {
            for (removed, text) in [
                (0, "\n"),
                (0, "x"),
                (0, "["),
                (0, "\""),
                (0, "\n# 🐧\n"),
                (1, ""),
                (0, "\r\n"),
            ] {
                let mut rope = Rope::from_str(&source);
                let mut highlighter = SyntaxHighlighter::new();
                highlighter.set_language(Some(&language()), &theme);
                parity(&mut highlighter, &rope, 0..rope.len_lines());
                highlighter.edit(&replace(
                    &mut rope,
                    offset,
                    (offset + removed).min(chars),
                    text,
                ));
                parity(&mut highlighter, &rope, 0..rope.len_lines());
                accepted += usize::from(!highlighter.tree_current);
            }
        }
    }
    assert!(
        accepted > 1_000,
        "guard must exercise line-local work, accepted {accepted}"
    );
}
#[test]
fn offscreen_paste_errors_repair_replacement_coalescing_and_cancellation_use_safe_fallbacks() {
    let source = (0..40)
        .map(|n| format!(":: block_{n}\n> line_{n}\n  Text 🦊.\n-> END\n"))
        .collect::<String>();
    let theme = EditorSyntaxTheme::default();
    let mut rope = Rope::from_str(&source);
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language()), &theme);
    parity(&mut highlighter, &rope, 0..4);
    let offset = rope.line_to_char(100);
    highlighter.edit(&replace(&mut rope, offset, offset, ":: broken [\n"));
    parity(&mut highlighter, &rope, 0..4);
    assert!(
        highlighter.tree_current,
        "offscreen malformed paste must fall back"
    );
    let end = offset + ":: broken [\n".chars().count();
    highlighter.edit(&replace(&mut rope, offset, end, ""));
    parity(&mut highlighter, &rope, 0..4);
    assert!(
        highlighter.tree_current,
        "repair must establish a new whole-file proof"
    );
    highlighter.edit(&replace(&mut rope, offset, offset, "\n"));
    parity(&mut highlighter, &rope, 0..4);
    assert!(!highlighter.tree_current);
    let mut blocks = SyntaxBlocks::default();
    highlighter.edit(&replace(&mut rope, offset, offset, "\n"));
    assert!(!highlighter.parse_cancellable(&rope, &mut blocks, &theme, 0..4, || true));
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
    highlighter.edit(&replace(&mut rope, offset, offset, "\n"));
    highlighter.edit(&replace(&mut rope, offset, offset, "\n"));
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
    let unknown = Rope::from_str(":: replacement\n> line\n  Different.\n");
    parity(&mut highlighter, &unknown, 0..unknown.len_lines());
    assert!(highlighter.tree_current);
    highlighter.set_language(Some(&language()), &theme);
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
}

#[test]
fn invalid_sources_retain_incremental_global_parsing_and_rust_remains_on_default_path() {
    let theme = EditorSyntaxTheme::default();
    let mut rope = Rope::from_str(":: broken [\n> line\n  Text.\n");
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language()), &theme);
    parity(&mut highlighter, &rope, 0..rope.len_lines());
    assert!(!highlighter.linewise.as_ref().unwrap().known_input(&rope));
    highlighter.edit(&replace(&mut rope, 0, 0, "# comment\n"));
    parity(&mut highlighter, &rope, 0..rope.len_lines());
    assert_eq!(
        highlighter.incremental_parses, 1,
        "line guard rejection must retain the edited global tree"
    );
    let rust = EditorLanguage::new(
        tree_sitter_rust::LANGUAGE,
        tree_sitter_rust::HIGHLIGHTS_QUERY,
    );
    highlighter.set_language(Some(&rust), &theme);
    assert!(highlighter.linewise.is_none());
}
#[test]
fn dark_viewport_reprojection_after_consecutive_unicode_edits_matches_full_parse() {
    let theme = EditorSyntaxTheme::dark();
    let mut rope = Rope::from_str(":: start\n> line\n  Café 🦊.\n-> END");
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language()), &theme);
    let mut actual = SyntaxBlocks::default();
    highlighter.parse(&rope, &mut actual, None, &theme);
    for text in ["\n", "# 🐧\n", "\n\n"] {
        highlighter.edit(&replace(&mut rope, 0, 0, text));
        highlighter.parse_visible(&rope, &mut actual, None, &theme, 0..2);
        assert!(!highlighter.tree_current);
        highlighter.highlight_rows(&rope, &mut actual, &theme, 2..rope.len_lines());
        let mut fresh = SyntaxHighlighter::new();
        fresh.set_language(Some(&language()), &theme);
        let mut expected = SyntaxBlocks::default();
        fresh.parse(&rope, &mut expected, None, &theme);
        for row in 2..rope.len_lines() {
            assert_eq!(describe(&actual, row), describe(&expected, row));
        }
    }
}

#[test]
fn structured_fixtures_and_cross_line_recovery_traces_match_full_highlighting() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let mut cases = Vec::new();
    for name in [
        "capture-values",
        "compact-diverts",
        "compact-markers",
        "escaped-braces",
        "id-recovery",
        "incomplete",
    ] {
        cases.push(
            std::fs::read_to_string(root.join(format!(
                "editors/recite-tree-sitter/test/fixtures/{name}.recite"
            )))
            .unwrap(),
        );
    }
    for path in [
        "language_pressure.recite",
        "core_language_spike.recite",
        "adapter_conformance/plural_runtime.recite",
    ] {
        cases.push(std::fs::read_to_string(root.join("fixtures/recite/valid").join(path)).unwrap());
    }
    // These fragments previously disproved unconditional line composition.
    for source in [
        r###"::
:ifx
:elsewhere
:matchmaking
:casefile
"###,
        r###"::
oops
:: start default
> intro_001@b3afea83efe6eb4b9f6a
  Parser recovery keeps this block available.
"###,
        r###":: start default
> plural_line@5fcf9a1f7b20211f4a92 bind=(count:int=["$count)
  You have one letter.
  | You have {count} letters.
-> END
"###,
    ] {
        cases.push(source.into());
    }
    let theme = EditorSyntaxTheme::default();
    for source in cases {
        for phase in 0..8 {
            let source = format!("{}{source}", "\n".repeat(phase));
            let original = Rope::from_str(&source);
            for offset in [0, original.len_chars() / 2, original.len_chars()] {
                for inserted in ["\n", "x", "\n! immediate call(\n", "\r\n"] {
                    let mut rope = original.clone();
                    let mut highlighter = SyntaxHighlighter::new();
                    highlighter.set_language(Some(&language()), &theme);
                    parity(&mut highlighter, &rope, 0..rope.len_lines());
                    highlighter.edit(&replace(&mut rope, offset, offset, inserted));
                    parity(&mut highlighter, &rope, 0..rope.len_lines());
                    highlighter.edit(&replace(
                        &mut rope,
                        offset,
                        offset + inserted.chars().count(),
                        "",
                    ));
                    parity(&mut highlighter, &rope, 0..rope.len_lines());
                }
            }
        }
    }
}

#[test]
fn large_clean_pastes_use_one_global_parse() {
    let theme = EditorSyntaxTheme::default();
    let mut rope = Rope::from_str(":: start\n> line\n  Original.\n-> END\n");
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language()), &theme);
    parity(&mut highlighter, &rope, 0..4);
    highlighter.edit(&replace(&mut rope, 0, 0, &"# pasted comment\n".repeat(300)));
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
    assert_eq!(highlighter.incremental_parses, 1);
}

#[path = "linewise/crlf.rs"]
mod crlf;
