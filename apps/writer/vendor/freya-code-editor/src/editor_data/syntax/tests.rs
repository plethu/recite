use super::*;

fn language() -> crate::languages::EditorLanguage {
    crate::languages::EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    )
}
fn settle(data: &mut CodeEditorData) {
    for _ in 0..10_000 {
        if !data.syntax_pending {
            break;
        }
        data.continue_syntax();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!data.syntax_pending, "background syntax never completed");
}
fn describe(line: &[(Color, crate::syntax::TextNode)]) -> Vec<String> {
    line.iter()
        .map(|(color, node)| match node {
            crate::syntax::TextNode::Range(range) => format!("{color:?}:{range:?}"),
            crate::syntax::TextNode::LineOfChars { len, char } => format!("{color:?}:{len}:{char}"),
        })
        .collect()
}
fn assert_fresh(data: &mut CodeEditorData) {
    settle(data);
    let mut reference = crate::syntax::SyntaxHighlighter::new();
    reference.set_language(data.language.as_ref(), &data.theme);
    let mut full = crate::syntax::SyntaxBlocks::default();
    reference.parse(&data.rope, &mut full, None, &data.theme);
    for row in data.visible_rows.clone() {
        if row >= data.rope.len_lines() {
            break;
        }
        assert_eq!(
            describe(data.metrics.syntax_blocks.get_line(row)),
            describe(full.get_line(row)),
            "row {row}"
        );
    }
    assert!(data.metrics.syntax_blocks.len() <= data.visible_rows.len());
    super::super::tests::check_widths(data);
}
#[test]
fn background_matches_full_highlights_through_replacements_and_line_breaks() {
    for source in [
        ":: start\n> id@11111111111111111111\n  Hello 🦊.\n-> END\n",
        "# a\r\nx\n",
        "# a\rx\nb",
        "# a\u{b}b\u{c}c\u{85}d\u{2028}e\u{2029}f",
    ] {
        let rope = Rope::from_str(source);
        for start in 0..=rope.len_chars() {
            let end = (start + 2).min(rope.len_chars());
            let mut data = CodeEditorData::new(rope.clone(), Some(language()));
            data.enable_background_highlighting().unwrap();
            assert_fresh(&mut data);
            let offset = rope.char_to_utf16_cu(start);
            data.remove(offset..rope.char_to_utf16_cu(end));
            data.insert("🦊\r\n  ", offset);
            data.parse();
            assert_fresh(&mut data);
        }
    }
}
#[test]
fn background_coalesces_edits_and_rejects_old_config_or_viewport_outputs() {
    let source = (0..1000)
        .map(|n| format!(":: block{n}\n> id@{n:020}\n  Hello 🦊 {{value}}.\n"))
        .collect::<String>();
    let mut data = CodeEditorData::new(Rope::from_str(&source), Some(language()));
    data.enable_background_highlighting().unwrap();
    for _ in 0..200 {
        let end = data.rope.len_utf16_cu();
        data.insert("x", end);
        data.parse();
    }
    data.visible_rows(100..250);
    data.set_theme(EditorSyntaxTheme::dark());
    data.parse();
    assert_fresh(&mut data);
    assert_eq!(
        data.metrics.syntax_blocks.len(),
        150,
        "tall viewports must not be truncated"
    );
    for _ in 0..200 {
        data.insert("x", 0);
    }
    assert!(data.pending_edits.is_none(), "overflow must remain cold");
    data.parse();
    assert_fresh(&mut data);
    data.undo();
    data.parse();
    assert_fresh(&mut data);
    data.redo();
    data.parse();
    assert_fresh(&mut data);
    data.rope = Rope::from_str(":: another\n  Current text.\n");
    data.parse();
    data.visible_rows(0..3);
    assert_fresh(&mut data);
}
#[test]
fn retained_syntax_rows_always_slice_current_text_after_unicode_newline_edits() {
    let source = (0..30)
        .map(|n| format!("# line{n} 🦊é\r\n"))
        .collect::<String>();
    let mut data = CodeEditorData::new(Rope::from_str(&source), Some(language()));
    data.enable_background_highlighting().unwrap();
    assert_fresh(&mut data);
    let old = data.rope.clone();
    let from = old.line_to_char(10) + 2;
    let to = old.line_to_char(12) + 3;
    data.remove(old.char_to_utf16_cu(from)..old.char_to_utf16_cu(to));
    data.insert("é🦊\n", data.rope.char_to_utf16_cu(from));
    for row in 0..data.rope.len_lines() {
        if let Some(line) = data.metrics.syntax_blocks.get_line_opt(row) {
            let text = line
                .iter()
                .map(|(_, n)| match n {
                    crate::syntax::TextNode::Range(r) => data.rope.slice(r.clone()).to_string(),
                    crate::syntax::TextNode::LineOfChars { len, .. } => " ".repeat(*len),
                })
                .collect::<String>();
            assert_eq!(
                text.trim_end_matches(['\r', '\n']),
                data.rope
                    .line(row)
                    .to_string()
                    .trim_end_matches(['\r', '\n'])
            );
        }
    }
    data.parse();
    assert_fresh(&mut data);
}

#[test]
fn unknown_rope_changes_break_edit_chains_before_or_after_recorded_edits() {
    for before in [true, false] {
        let mut data =
            CodeEditorData::new(Rope::from_str(":: old\n  Original.\n"), Some(language()));
        data.enable_background_highlighting().unwrap();
        assert_fresh(&mut data);
        if before {
            data.rope = Rope::from_str(":: new\n  🦊 Current.\n");
        }
        data.insert("x", 0);
        if !before {
            data.rope = Rope::from_str(":: replaced\n  Last.\n");
        }
        data.parse();
        assert_fresh(&mut data);
    }
}
#[test]
fn output_cannot_publish_after_an_unsubmitted_mutation() {
    let source = (0..1000)
        .map(|n| format!(":: block{n}\n  Source.\n"))
        .collect::<String>();
    let mut data = CodeEditorData::new(Rope::from_str(&source), Some(language()));
    data.enable_background_highlighting().unwrap();
    data.insert("🦊", 0);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!data.continue_syntax());
    assert!(!data.syntax_pending);
    assert!(data.metrics.syntax_blocks.is_empty());
    data.parse();
    assert_fresh(&mut data);
    data.parse();
    data.rope = Rope::from_str("# replaced without parse");
    std::thread::sleep(Duration::from_millis(100));
    assert!(!data.syntax_rows_current());
    assert!(!data.continue_syntax());
    assert!(!data.syntax_pending);
    data.parse();
    assert_fresh(&mut data);
}

#[test]
fn shared_grammar_fixtures_match_full_highlights_at_all_viewports() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../editors/recite-tree-sitter/test/fixtures");
    let mut paths = std::fs::read_dir(fixtures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 6);
    for path in paths {
        let source = std::fs::read_to_string(&path).unwrap();
        let mut data = CodeEditorData::new(Rope::from_str(&source), Some(language()));
        data.enable_background_highlighting().unwrap();
        for first in (0..data.rope.len_lines()).step_by(3) {
            data.visible_rows(first..(first + 5).min(data.rope.len_lines()));
            assert_fresh(&mut data);
        }
        data.insert("# 🦊\r\n", 0);
        data.parse();
        data.visible_rows(0..data.rope.len_lines());
        assert_fresh(&mut data);
    }
}

#[test]
fn failed_worker_clears_provisional_rows_and_can_be_explicitly_restarted() {
    let mut data = CodeEditorData::new(Rope::from_str("# initial\n"), Some(language()));
    data.enable_background_highlighting().unwrap();
    assert_fresh(&mut data);
    data.background.as_ref().unwrap().close();
    data.parse();
    settle(&mut data);
    assert!(data.syntax_failed);
    assert!(data.metrics.syntax_blocks.is_empty());
    data.insert("x", 0);
    data.parse();
    assert!(!data.syntax_pending);
    assert!(data.pending_edits.is_none());
    assert!(data.metrics.syntax_blocks.is_empty());
    data.set_language(Some(language()));
    data.enable_background_highlighting().unwrap();
    assert_fresh(&mut data);
    assert!(!data.syntax_failed);
}

#[test]
fn exhausted_history_preserves_settled_and_pending_highlighting() {
    let mut data = CodeEditorData::new(Rope::from_str("# unchanged\n"), Some(language()));
    data.enable_background_highlighting().unwrap();
    for pending in [true, false] {
        if !pending {
            assert_fresh(&mut data);
        }
        let previous = data.submitted.clone().unwrap();
        let rows = data.metrics.syntax_blocks.len();
        assert!(data.undo().is_none());
        assert!(data.redo().is_none());
        assert_eq!(data.syntax_pending, pending);
        assert!(std::sync::Arc::ptr_eq(
            data.submitted.as_ref().unwrap(),
            &previous
        ));
        assert_eq!(data.metrics.syntax_blocks.len(), rows);
    }
}

#[test]
fn replacement_retry_and_prose_reject_old_worker_notifications() {
    let mut data = CodeEditorData::new(Rope::from_str("# first"), Some(language()));
    assert!(data.syntax_notifications().is_none());
    data.enable_background_highlighting().unwrap();
    let first = data.syntax_notifications().unwrap();
    data = CodeEditorData::new(Rope::from_str("# replacement"), Some(language()));
    data.enable_background_highlighting().unwrap();
    let second = data.syntax_notifications().unwrap();
    assert!(!data.receive_syntax(&first, false));
    assert!(!data.receive_syntax(&first, true));
    assert_fresh(&mut data);
    data.background.as_ref().unwrap().close();
    assert!(data.receive_syntax(&second, true));
    assert!(data.syntax_failed);
    assert!(data.syntax_notifications().is_none());
    data.enable_background_highlighting().unwrap();
    assert!(!data.receive_syntax(&second, false));
    assert!(!data.receive_syntax(&second, true));
    assert_fresh(&mut data);
    data = CodeEditorData::new(Rope::from_str("prose"), None);
    assert!(data.syntax_notifications().is_none());
    assert!(!data.receive_syntax(&first, true));
    assert!(!data.receive_syntax(&second, false));
}

#[test]
fn a_stale_notification_does_not_prevent_later_current_publication() {
    let mut data = CodeEditorData::new(Rope::from_str("# first"), Some(language()));
    data.enable_background_highlighting().unwrap();
    let receiver = data.syntax_notifications().unwrap();
    for _ in 0..10_000 {
        if !receiver.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    data.insert("🦊", 0);
    assert!(receiver.try_recv().is_ok());
    assert!(!data.receive_syntax(&receiver, false));
    data.parse();
    for _ in 0..10_000 {
        if receiver.try_recv().is_ok() && data.receive_syntax(&receiver, false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!data.syntax_pending);
    assert_fresh(&mut data);
}

#[test]
fn unavailable_background_mode_does_not_fall_back_to_full_foreground_parsing() {
    let mut data = CodeEditorData::new(
        Rope::from_str(&"# large source\n".repeat(100_000)),
        Some(language()),
    );
    data.fail_syntax();
    assert!(data.background.is_none());
    data.insert("🦊", 0);
    data.parse();
    assert!(data.syntax_failed);
    assert!(data.metrics.syntax_blocks.is_empty());
    assert!(data.syntax_notifications().is_none());
    data.enable_background_highlighting().unwrap();
    assert_fresh(&mut data);
}

#[test]
fn equal_themes_preserve_pending_and_settled_syntax() {
    let mut data = CodeEditorData::new(Rope::from_str("# current\n"), Some(language()));
    data.enable_background_highlighting().unwrap();
    for pending in [true, false] {
        if !pending {
            assert_fresh(&mut data);
        }
        let submitted = data.submitted.clone().unwrap();
        let rows = data.metrics.syntax_blocks.len();
        data.set_theme(data.theme.clone());
        assert_eq!(data.syntax_pending, pending);
        assert!(std::sync::Arc::ptr_eq(
            data.submitted.as_ref().unwrap(),
            &submitted
        ));
        assert_eq!(data.metrics.syntax_blocks.len(), rows);
        assert!(data.pending_edits.as_ref().is_some_and(Vec::is_empty));
    }
    data.insert("x", 1);
    data.set_theme(data.theme.clone());
    assert_eq!(data.pending_edits.as_ref().unwrap().len(), 1);
    let mut changed = data.theme.clone();
    changed.comment = Color::RED;
    data.set_theme(changed);
    assert!(data.syntax_pending);
    assert_fresh(&mut data);
    assert_eq!(data.metrics.syntax_blocks.get_line(0)[0].0, Color::RED);
}
