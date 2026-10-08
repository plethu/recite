use proptest::prelude::*;
use recite_core::{
    DiagnosticArgumentValue,
    ast::{ConditionExpression, Statement},
};
use recite_parser::parse;

const PATH: &str = "conditions.recite";

proptest! {
    #[test]
    fn mixed_nesting_budget_is_independent_of_boolean_width_and_line_endings(
        wrappers in prop::collection::vec(any::<bool>(), 0..141),
        width in 1usize..48,
        conjunction in any::<bool>(),
        ending in prop_oneof![Just("\n"), Just("\r"), Just("\r\n")],
    ) {
        let operator = if conjunction { " and " } else { " or " };
        let mut expression = vec!["ready()"; width].join(operator);
        for grouped in &wrappers {
            expression = if *grouped { format!("({expression})") } else { format!("not {expression}") };
        }
        let source = format!("{}\n:: after\n> after@22222222222222222222\n  🦀 Still here.\n-> END\n", conditional_source(&expression))
            .replace('\n', ending);
        let parsed = parse(PATH, &source);
        prop_assert_eq!(parsed.syntax().text().to_string(), source.clone());
        let lowered = parsed.lower_source_file();
        prop_assert_eq!(lowered.source_file.blocks.len(), 2);
        prop_assert_eq!(lowered.source_file.blocks[1].id.as_str(), "after");
        if wrappers.len() <= 128 {
            prop_assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
            let Statement::If(branch) = &lowered.source_file.blocks[0].statements[0] else {
                return Err(TestCaseError::fail("generated condition must lower to an if branch"));
            };
            let mut expressions = vec![&branch.condition];
            let mut calls = 0;
            let mut unary = 0;
            while let Some(expression) = expressions.pop() {
                match expression {
                    ConditionExpression::Call(call) => {
                        prop_assert_eq!(call.function.as_str(), "ready");
                        prop_assert!(call.args.is_empty());
                        calls += 1;
                    }
                    ConditionExpression::Not(inner) | ConditionExpression::Grouped(inner) => {
                        unary += 1;
                        expressions.push(&inner.expression);
                    }
                    ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                        prop_assert_eq!(matches!(expression, ConditionExpression::And(_)), conjunction);
                        expressions.extend(&group.expressions);
                    }
                }
            }
            prop_assert_eq!(calls, width);
            prop_assert_eq!(unary, wrappers.len());
            prop_assert_eq!(lowered.source_file.clone(), lowered.source_file);
        } else {
            prop_assert_eq!(lowered.diagnostics[0].code.as_str(), "RECITE_PARSE013");
            prop_assert_eq!(lowered.diagnostics[0].span.start.line(), 2);
            prop_assert!(lowered.diagnostics[0].record().is_ok());
        }
    }
}

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
