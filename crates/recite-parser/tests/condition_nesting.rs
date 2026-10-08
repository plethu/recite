use recite_core::{DiagnosticArgumentValue, ast::Statement};
use recite_parser::parse;

const PATH: &str = "conditions.recite";

#[test]
fn condition_syntax_accepts_the_nesting_boundary() {
    let mut boolean_groups = "ready()".to_owned();
    for _ in 0..128 {
        boolean_groups = format!("(ready() or ready() and {boolean_groups})");
    }
    for expression in [
        format!("{}ready()", "not ".repeat(128)),
        format!("{}ready(){}", "(".repeat(128), ")".repeat(128)),
        format!("{}ready(){}", "not (".repeat(64), ")".repeat(64)),
        boolean_groups,
    ] {
        let lowered = parse(PATH, conditional_source(&expression)).lower_source_file();
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        assert!(matches!(
            lowered.source_file.blocks[0].statements[0],
            Statement::If(_)
        ));
        assert_eq!(lowered.source_file.clone(), lowered.source_file);
    }
}

#[test]
fn excessive_unary_grouped_and_mixed_conditions_have_typed_spanned_diagnostics() {
    for (expression, column) in [
        (format!("{}ready()", "not ".repeat(129)), 517),
        (
            format!("{}ready(){}", "(".repeat(129), ")".repeat(129)),
            133,
        ),
        (
            format!("{}not ready(){}", "not (".repeat(64), ")".repeat(64)),
            325,
        ),
    ] {
        let lowered = parse(PATH, conditional_source(&expression)).lower_source_file();
        let diagnostic = &lowered.diagnostics[0];
        assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE013");
        assert_eq!(diagnostic.span.file, PATH);
        assert_eq!(
            (diagnostic.span.start.line(), diagnostic.span.start.column()),
            (2, column)
        );
        let presentation = diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation");
        assert_eq!(
            presentation.id().as_str(),
            "diagnostic-parse-013-nesting-limit"
        );
        assert_eq!(
            presentation.arguments().get("limit"),
            Some(&DiagnosticArgumentValue::Integer(128))
        );
        let record = diagnostic.record().expect("recordable nesting diagnostic");
        assert_eq!(record.span, diagnostic.span);
        assert_eq!(record.presentation, *presentation);
    }
}

#[test]
fn wide_boolean_groups_and_argument_lists_remain_shallow() {
    for expression in [
        vec!["ready()"; 10_000].join(" and "),
        vec!["ready()"; 10_000].join(" or "),
        format!("ready({})", vec!["1"; 10_000].join(", ")),
    ] {
        let lowered = parse(PATH, conditional_source(&expression)).lower_source_file();
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        assert!(matches!(
            lowered.source_file.blocks[0].statements[0],
            Statement::If(_)
        ));
    }
}

#[test]
fn condition_nesting_failures_recover_at_following_blocks_and_keep_shallow_errors() {
    let source = format!(
        "{}\n:: after\n> after@22222222222222222222\n  Still here.\n-> END\n",
        conditional_source(&format!("{}ready()", "not ".repeat(10_000))),
    );
    let lowered = parse(PATH, &source).lower_source_file();
    assert_eq!(lowered.source_file.blocks.len(), 2);
    assert_eq!(lowered.source_file.blocks[1].id.as_str(), "after");
    assert!(matches!(
        lowered.source_file.blocks[1].statements[0],
        Statement::Line(_)
    ));
    assert_eq!(lowered.diagnostics[0].code.as_str(), "RECITE_PARSE013");
    let malformed = parse(PATH, conditional_source("not (ready(")).lower_source_file();
    let presentation = malformed.diagnostics[0]
        .presentation
        .as_ref()
        .expect("syntax presentation");
    assert_eq!(presentation.id().as_str(), "diagnostic-parse-013");
    assert_eq!(
        presentation.arguments().get("reason"),
        Some(&DiagnosticArgumentValue::String(
            "expected_scalar_argument".to_owned()
        ))
    );
}

fn conditional_source(expression: &str) -> String {
    format!(
        ":: start default\n:if {expression}\n  > line@11111111111111111111\n    Hello.\n-> END\n"
    )
}
