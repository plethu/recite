use super::*;

pub(super) fn lower(source: &str) -> LoweredSourceFile {
    parse(TEST_PATH, source).lower_source_file()
}

pub(super) fn single_block(lowered: &LoweredSourceFile) -> &Block {
    assert_eq!(lowered.source_file.blocks.len(), 1);
    &lowered.source_file.blocks[0]
}

pub(super) fn line_statement(block: &Block, index: usize) -> &Line {
    let Statement::Line(line) = &block.statements[index] else {
        panic!("expected statement {index} to be a line");
    };

    line
}

pub(super) fn choice_statement(block: &Block, index: usize) -> &Choice {
    let Statement::Choice(choice) = &block.statements[index] else {
        panic!("expected statement {index} to be a choice");
    };

    choice
}

pub(super) fn nested_choice(line: &Line, index: usize) -> &Choice {
    let Statement::Choice(choice) = &line.statements[index] else {
        panic!("expected nested statement {index} to be a choice");
    };

    choice
}

pub(super) fn if_statement(block: &Block, index: usize) -> &IfBranch {
    let Statement::If(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be an if branch");
    };

    branch
}

pub(super) fn match_statement(block: &Block, index: usize) -> &MatchBranch {
    let Statement::Match(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be a match branch");
    };

    branch
}

pub(super) fn comment_statement(block: &Block, index: usize) -> &recite_core::Comment {
    let Statement::Comment(comment) = &block.statements[index] else {
        panic!("expected statement {index} to be a comment");
    };

    comment
}

pub(super) fn assert_diagnostic_codes<const N: usize>(
    lowered: &LoweredSourceFile,
    expected: [&str; N],
) {
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

pub(super) fn diagnostic_snapshot_name(source_path: &str) -> String {
    fixture_support::fixture_snapshot_name(source_path, ".diagnostics.txt")
}

pub(super) fn lowered_snapshot_name(source_path: &str) -> String {
    fixture_support::fixture_snapshot_name(source_path, ".lowered.txt")
}

pub(super) fn lowered_summary(lowered: &LoweredSourceFile) -> String {
    let mut summary = String::new();

    summary.push_str("diagnostics:\n");
    if lowered.diagnostics.is_empty() {
        summary.push_str("  <none>\n");
    } else {
        for diagnostic in &lowered.diagnostics {
            summary.push_str(&format!(
                "  - {} @ {}:{}\n",
                diagnostic.code.as_str(),
                diagnostic.span.start.line(),
                diagnostic.span.start.column()
            ));
        }
    }

    summary.push_str("blocks:\n");
    for block in &lowered.source_file.blocks {
        summary.push_str(&format!(
            "  - {} default={} statements={}\n",
            block.id.as_str(),
            block.is_default,
            block.statements.len()
        ));

        for statement in &block.statements {
            match statement {
                Statement::Comment(comment) => summary.push_str(&format!(
                    "    - comment {:?} @ {}:{}\n",
                    comment.text,
                    comment.span.start.line(),
                    comment.span.start.column()
                )),
                Statement::Line(line) => summary.push_str(&format!(
                    "    - line {} speaker={} text={:?} metadata=[{}]\n",
                    source_id_summary(&line.source_id),
                    line.speaker
                        .as_ref()
                        .map(SpeakerId::as_str)
                        .unwrap_or("<none>"),
                    line.source_text.text,
                    line.metadata
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                other => summary.push_str(&format!("    - {:?}\n", other.kind())),
            }
        }
    }

    summary
}

pub(super) fn lowered_fixture_summary(lowered: &LoweredSourceFile) -> String {
    let mut summary = format!("source_file: {}\nblocks:\n", lowered.source_file.path);

    for block in &lowered.source_file.blocks {
        summary.push_str(&format!(
            "- block {} default={} speaker={} metadata=[{}]\n",
            block.id.as_str(),
            block.is_default,
            block
                .default_speaker
                .as_ref()
                .map(SpeakerId::as_str)
                .unwrap_or("<none>"),
            metadata_keys(&block.metadata)
        ));
        push_statement_summaries(&mut summary, &block.statements, 1);
    }

    summary
}

fn push_statement_summaries(summary: &mut String, statements: &[Statement], depth: usize) {
    for statement in statements {
        push_statement_summary(summary, statement, depth);
    }
}

fn push_statement_summary(summary: &mut String, statement: &Statement, depth: usize) {
    let indent = "  ".repeat(depth);
    match statement {
        Statement::Comment(comment) => {
            summary.push_str(&format!(
                "{indent}- comment {:?} @ {}:{}\n",
                comment.text,
                comment.span.start.line(),
                comment.span.start.column()
            ));
        }
        Statement::Line(line) => {
            summary.push_str(&format!(
                "{indent}- line {} speaker={} text={:?} metadata=[{}]\n",
                source_id_summary(&line.source_id),
                line.speaker
                    .as_ref()
                    .map(SpeakerId::as_str)
                    .unwrap_or("<none>"),
                line.source_text.text,
                metadata_keys(&line.metadata)
            ));
            push_statement_summaries(summary, &line.statements, depth + 1);
        }
        Statement::Choice(choice) => {
            summary.push_str(&format!(
                "{indent}- choice {} text={:?} target={} metadata=[{}]\n",
                source_id_summary(&choice.source_id),
                choice.source_text.text,
                choice
                    .target
                    .as_ref()
                    .map(|target| divert_target_summary(&target.target))
                    .unwrap_or_else(|| "<none>".to_owned()),
                metadata_keys(&choice.metadata)
            ));
            push_statement_summaries(summary, &choice.statements, depth + 1);
        }
        Statement::Divert(divert) => {
            summary.push_str(&format!(
                "{indent}- divert {}\n",
                divert_target_summary(&divert.target)
            ));
        }
        Statement::If(branch) => {
            summary.push_str(&format!(
                "{indent}- if {}\n",
                condition_summary(&branch.condition)
            ));
            if !branch.then_statements.is_empty() {
                summary.push_str(&format!("{indent}  then:\n"));
                push_statement_summaries(summary, &branch.then_statements, depth + 2);
            }
            if !branch.else_statements.is_empty() {
                summary.push_str(&format!("{indent}  else:\n"));
                push_statement_summaries(summary, &branch.else_statements, depth + 2);
            }
        }
        Statement::Match(branch) => {
            summary.push_str(&format!(
                "{indent}- match {}\n",
                condition_call_summary(&branch.scrutinee)
            ));
            for arm in &branch.arms {
                summary.push_str(&format!(
                    "{indent}  case {}:\n",
                    match_pattern_summary(&arm.pattern)
                ));
                push_statement_summaries(summary, &arm.statements, depth + 2);
            }
        }
        Statement::Effect(effect) => {
            summary.push_str(&format!(
                "{indent}- effect {:?} {}({})\n",
                effect.mode,
                effect.function,
                effect
                    .args
                    .iter()
                    .map(argument_summary)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
}

fn metadata_keys(metadata: &recite_core::SourceMetadata) -> String {
    metadata
        .iter()
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn source_id_summary(source_id: &recite_core::SourceId) -> String {
    match source_id {
        recite_core::SourceId::Missing => "<missing>".to_owned(),
        recite_core::SourceId::Draft { label } => format!("{label}@"),
        recite_core::SourceId::Frozen { label, anchor } => {
            format!("{label}@{}", anchor.as_str())
        }
        recite_core::SourceId::Malformed { raw } => format!("<malformed {raw:?}>"),
    }
}

fn divert_target_summary(target: &DivertTarget) -> String {
    match target {
        DivertTarget::End => recite_core::END_DIVERT_TARGET.to_owned(),
        DivertTarget::Block(reference) => match &reference.file {
            Some(file) => format!("{file}::{}", reference.block_id),
            None => reference.block_id.to_string(),
        },
    }
}

fn condition_summary(condition: &ConditionExpression) -> String {
    match condition {
        ConditionExpression::Call(call) => condition_call_summary(call),
        ConditionExpression::And(group) => condition_group_summary("and", &group.expressions),
        ConditionExpression::Or(group) => condition_group_summary("or", &group.expressions),
        ConditionExpression::Not(unary) => format!("not {}", condition_summary(&unary.expression)),
        ConditionExpression::Grouped(unary) => {
            format!("({})", condition_summary(&unary.expression))
        }
    }
}

fn condition_group_summary(operator: &str, expressions: &[ConditionExpression]) -> String {
    expressions
        .iter()
        .map(condition_summary)
        .collect::<Vec<_>>()
        .join(&format!(" {operator} "))
}

fn condition_call_summary(call: &recite_core::ConditionCall) -> String {
    format!(
        "{}({})",
        call.function,
        call.args
            .iter()
            .map(argument_summary)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn argument_summary(argument: &Argument) -> String {
    match argument {
        Argument::Identifier(identifier) => identifier.clone(),
        Argument::Value(value) => scalar_summary(value),
    }
}

fn scalar_summary(value: &ScalarValue) -> String {
    match value {
        ScalarValue::String(value) => format!("{value:?}"),
        ScalarValue::Integer(value) => value.to_string(),
        ScalarValue::Float(value) => value.to_string(),
        ScalarValue::Boolean(value) => value.to_string(),
    }
}

fn match_pattern_summary(pattern: &MatchPattern) -> String {
    match pattern {
        MatchPattern::Variant(variant) => variant.clone(),
        MatchPattern::Wildcard => "_".to_owned(),
    }
}
