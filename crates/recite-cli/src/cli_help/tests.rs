use super::grouped_command;
use unicode_width::UnicodeWidthStr;

#[test]
fn grouped_help_wraps_descriptions_with_continuation_indentation() {
    let about = "Inspect the schema produced by a provider and report its structured metadata.";
    for columns in [58, 80, 100] {
        let text = grouped_command("inspect-schema", about, 31, columns);
        assert!(text.lines().all(|line| line.width() <= columns));
        assert!(text.lines().skip(1).all(|line| line.starts_with("    ")));
        assert!(
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .ends_with(about)
        );
    }
}
