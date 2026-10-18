use super::*;

fn tree(source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&recite_writer_grammar::LANGUAGE.into())
        .unwrap();
    parser.parse(source, None).unwrap()
}
#[test]
fn metadata_eligibility_matches_scanning_all_supported_and_rejected_breaks() {
    let breaks = [
        "", "\n", "\r\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}",
    ];
    for separator in &breaks[1..] {
        assert_eq!(
            Rope::from_str(separator).len_lines(),
            2,
            "Ropey line configuration: {separator:?}"
        );
    }
    for first in breaks {
        for second in breaks {
            for phase in 0..8 {
                let source = format!(
                    "{}# {}{first}:: start default{second}> line@11111111111111111111\r\n  Café 🦊.{first}-> END{second}",
                    "\r\n".repeat(phase),
                    "é🦊".repeat(500),
                );
                let rope = Rope::from_str(&source);
                assert_eq!(
                    supported_input(&rope, &tree(&source)),
                    supported(rope.chars()),
                    "{first:?}, {second:?}, phase={phase}"
                );
            }
        }
    }
    for source in [
        "",
        " ",
        "\t",
        "\n\r\n",
        ":: start default\n-> END",
        ":: start default\r\n-> END\r\n   ",
    ] {
        let rope = Rope::from_str(source);
        assert_eq!(
            supported_input(&rope, &tree(source)),
            supported(rope.chars()),
            "{source:?}"
        );
    }
}
#[test]
fn incomplete_root_coverage_preserves_the_checked_whitespace_fallback() {
    let source = ":: start default\n-> END\n";
    let tree = tree(source);
    for suffix in [" ", "\n", "\r\n", "\r", "\u{b}", "\u{c}"] {
        let rope = Rope::from_str(&format!("{source}{suffix}"));
        assert_eq!(
            clean(&tree, &rope),
            suffix.chars().all(|c| c.is_ascii_whitespace())
        );
        assert!(tree.root_node().end_byte() < rope.len_bytes());
        assert_eq!(
            supported_input(&rope, &tree),
            supported(rope.chars()),
            "{suffix:?}"
        );
    }
}
