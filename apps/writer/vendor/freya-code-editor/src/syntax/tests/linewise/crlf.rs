use super::*;
const SOURCE: &str = ":: start default\r\n\r\n> line@11111111111111111111\r\n  Café 🦊.\r\n-> END";
fn seeded(source: &str) -> (Rope, SyntaxHighlighter) {
    let rope = Rope::from_str(source);
    let mut highlighter = SyntaxHighlighter::new();
    highlighter.set_language(Some(&language()), &EditorSyntaxTheme::default());
    parity(&mut highlighter, &rope, 0..rope.len_lines());
    assert!(highlighter.linewise.as_ref().unwrap().known_input(&rope));
    (rope, highlighter)
}
fn changed(source: &str, from: usize, to: usize, text: &str, fast: bool) {
    let (mut rope, mut highlighter) = seeded(source);
    let parses = highlighter.incremental_parses;
    highlighter.edit(&replace(&mut rope, from, to, text));
    parity(&mut highlighter, &rope, 0..rope.len_lines());
    assert_eq!(
        !highlighter.tree_current, fast,
        "{source:?}, {from}..{to}, {text:?}"
    );
    if fast {
        assert_eq!(highlighter.incremental_parses, parses);
        assert!(highlighter.linewise.as_ref().unwrap().known_input(&rope));
    }
}
#[test]
fn crlf_and_mixed_sources_keep_unicode_spans_and_use_line_local_work() {
    let structured = ":: start default\n> line@11111111111111111111\n  Café 🦊 {value}.\n? reply\n  Choice.\n  -> next\n:: next\n! immediate call()\n-> END";
    for phase in 0..8 {
        for mixed in [false, true] {
            let mut source = "\r\n".repeat(phase);
            source.push_str(&format!("# {}\r\n", "x".repeat(1100)));
            for (index, line) in structured.split('\n').enumerate() {
                if index > 0 {
                    source.push_str(if mixed && index % 2 == 0 {
                        "\n"
                    } else {
                        "\r\n"
                    });
                }
                source.push_str(line);
            }
            let offset = source[..source.find("Café").unwrap()].chars().count();
            changed(&source, offset, offset, "é", true);
        }
    }
}
#[test]
fn complete_pairs_and_lf_conversion_remain_fast_but_pair_damage_falls_back() {
    let pair = SOURCE.find('\r').unwrap(); // ASCII prefix, so also the char offset.
    changed(SOURCE, 0, 0, "\r\n", true);
    changed(SOURCE, pair + 2, pair + 4, "", true); // Remove a blank physical line.
    changed(SOURCE, pair, pair + 1, "", true); // CRLF -> LF.
    let lf = SOURCE.replacen("\r\n", "\n", 1);
    changed(&lf, pair, pair, "\r", true); // LF -> CRLF.
    changed(SOURCE, pair + 1, pair + 1, "x", false);
    changed(SOURCE, pair + 1, pair + 2, "", false);
}
#[test]
fn offscreen_pair_damage_and_repair_require_a_new_full_parse_before_reuse() {
    let source = (0..40)
        .map(|n| format!(":: block_{n}\r\n> line_{n}\r\n  Café 🦊.\r\n-> END\r\n"))
        .collect::<String>();
    let (mut rope, mut highlighter) = seeded(&source);
    let cr = rope.line_to_char(100) + rope.line(100).len_chars() - 2;
    assert_eq!(rope.char(cr), '\r');
    highlighter.edit(&replace(&mut rope, cr + 1, cr + 2, ""));
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
    assert!(!highlighter.linewise.as_ref().unwrap().known_input(&rope));
    highlighter.edit(&replace(&mut rope, cr + 1, cr + 1, "\n"));
    parity(&mut highlighter, &rope, 0..4);
    assert!(highlighter.tree_current);
    assert!(highlighter.linewise.as_ref().unwrap().known_input(&rope));
    let parses = highlighter.incremental_parses;
    highlighter.edit(&replace(&mut rope, 0, 0, "\r\n"));
    parity(&mut highlighter, &rope, 0..4);
    assert!(!highlighter.tree_current);
    assert_eq!(highlighter.incremental_parses, parses);
}
