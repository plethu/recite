use super::*;

#[test]
fn condition_precedence_and_grouping_are_lowered() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_a() or knows_b() and not (blocked())\n",
        "  > gated_line@76f3e924707c7391ada3\n",
        "    Hi.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let branch = if_statement(single_block(&lowered), 0);
    let ConditionExpression::Or(or_group) = &branch.condition else {
        panic!("expected top-level or condition");
    };
    assert_eq!(or_group.expressions.len(), 2);
    let ConditionExpression::And(and_group) = &or_group.expressions[1] else {
        panic!("expected and to bind tighter than or");
    };
    assert_eq!(and_group.expressions.len(), 2);
    let ConditionExpression::Not(not) = &and_group.expressions[1] else {
        panic!("expected not expression");
    };
    assert!(matches!(
        not.expression.as_ref(),
        ConditionExpression::Grouped(_)
    ));
}

#[test]
fn condition_parser_rejects_dangling_and_trailing_tokens() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player) trailing\n",
        "  > trailing_tokens@f7bdc23ba7a5cb2b29f6\n",
        "    Hi.\n",
        ":if knows_a() and\n",
        "  > dangling_operator@2bf645e52fce99236c14\n",
        "    Hi.\n",
        "? ask@9ef6f5db414aeedfe60e if knows_secret(player) sfx=door\n",
        "  Ask.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE013", "RECITE_PARSE013", "RECITE_PARSE018"],
    );
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.span.start.line(), diagnostic.span.start.column()))
            .collect::<Vec<_>>(),
        [(2, 26), (5, 18), (8, 28)]
    );

    let choice = choice_statement(single_block(&lowered), 0);
    assert_eq!(
        choice.id.as_ref().map(recite_core::ChoiceId::as_str),
        Some("9ef6f5db414aeedfe60e")
    );
    assert!(choice.availability_requirement.is_none());
    assert!(choice.metadata.is_empty());
}

#[test]
fn effect_parser_rejects_incomplete_or_trailing_calls() {
    let source = concat!(
        ":: tavern_arrival\n",
        "! deferred play_sfx(snap) trailing\n",
        "! immediate play_sfx(\n",
        "! blocking\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE012", "RECITE_PARSE012", "RECITE_PARSE012"],
    );
    assert!(single_block(&lowered).statements.is_empty());
}

#[test]
fn else_only_attaches_to_immediately_preceding_if() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        "  > gated_line@b6b26d29e5133c89c4d4\n",
        "    Hi.\n",
        "# comment breaks the if/else adjacency\n",
        ":else\n",
        "  > fallback_line@5cf66b61dd11e6cdecad\n",
        "    Hi.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE015"]);
    let block = single_block(&lowered);
    assert_eq!(
        block
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::If, StatementKind::Comment]
    );
    assert!(if_statement(block, 0).else_statements.is_empty());
}

#[test]
fn nested_if_else_and_match_bodies_keep_their_owners() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if outer()\n",
        "  :if inner()\n",
        "    > inner_true@a72fbfd57f1d43d0ea9a\n",
        "      Inner true.\n",
        "  :else\n",
        "    :match stage(thread)\n",
        "      :case ready\n",
        "        > ready_line@440e60c8a3277595b5ee\n",
        "          Ready.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let outer = if_statement(single_block(&lowered), 0);
    assert!(outer.else_statements.is_empty());
    assert_eq!(outer.then_statements.len(), 1);

    let Statement::If(inner) = &outer.then_statements[0] else {
        panic!("expected nested if");
    };
    assert_eq!(inner.then_statements.len(), 1);
    assert_eq!(inner.else_statements.len(), 1);

    let Statement::Match(branch) = &inner.else_statements[0] else {
        panic!("expected match inside inner else");
    };
    assert_eq!(branch.arms.len(), 1);
    assert_eq!(
        branch.arms[0].pattern,
        MatchPattern::Variant("ready".to_owned())
    );
}

#[test]
fn empty_if_and_match_bodies_lower_without_panics() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        ":match thread_stage(thread)\n",
        "> after_empty_bodies@1922554c3f1ad69fdfd2\n",
        "  Carry on.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let block = single_block(&lowered);
    assert_eq!(
        block
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::If, StatementKind::Match, StatementKind::Line]
    );
    assert!(if_statement(block, 0).then_statements.is_empty());
    assert!(match_statement(block, 1).arms.is_empty());
}

#[test]
fn case_extra_token_diagnostic_points_at_extra_field() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":match thread_stage(thread)\n",
        "  :case tired extra\n",
        "    > tired_line@49f1bd4973f3105dba9f\n",
        "      Tired.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE014"]);
    assert_eq!(lowered.diagnostics[0].span.start.line(), 3);
    assert_eq!(lowered.diagnostics[0].span.start.column(), 15);
    assert!(match_statement(single_block(&lowered), 0).arms.is_empty());
}

#[test]
fn malformed_headers_conditions_and_cases_report_diagnostics() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> line@82cc7e460b11d88c0c11 bare key=\n",
        "  Hello.\n",
        "? if@cd8afc035fbf1f835d87 knows_secret(player)\n",
        "  Choice.\n",
        "->\n",
        "! delayed play()\n",
        ":if knows_secret(\n",
        "  > gated@701cf2ef48bb76b67e9a\n",
        "    Hi.\n",
        ":else trailing\n",
        "  > fallback@7dec7fac028a4505641a\n",
        "    Hi.\n",
        ":case tired\n",
        "  > orphan@a5937c770583e84455ea\n",
        "    No.\n",
        ":match thread_stage(thread)\n",
        "  > not_case@821f6c84f87f021a712e\n",
        "    No.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE010",
            "RECITE_PARSE012",
            "RECITE_PARSE013",
            "RECITE_PARSE008",
            "RECITE_PARSE016",
            "RECITE_PARSE014",
        ],
    );
}

#[test]
fn conditional_body_is_not_flattened_into_block_statements() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        "  > gated_line@0585f40c4c005b954076\n",
        "    You know the password.\n",
        "> ordinary_line@213e9ffd21d449db603c\n",
        "  Welcome.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 2);

    let branch = if_statement(single_block(&lowered), 0);
    assert_eq!(branch.then_statements.len(), 1);
    let Statement::Line(gated_line) = &branch.then_statements[0] else {
        panic!("expected gated line");
    };
    assert_eq!(
        gated_line.id.as_ref().map(recite_core::LineId::as_str),
        Some("0585f40c4c005b954076")
    );

    let line = line_statement(single_block(&lowered), 1);
    assert_eq!(
        line.id.as_ref().map(recite_core::LineId::as_str),
        Some("213e9ffd21d449db603c")
    );
    assert_eq!(line.source_text.text, "Welcome.");
}

#[test]
fn nested_choice_body_is_not_appended_to_parent_prose() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> prompt_line@5588327e98b7326b9c71\n",
        "  What do you need?\n",
        "  ? ask_road@ff45a305ae9bb621ed7a\n",
        "    Ask about the road.\n",
        "  Still parent prose.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE017"]);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(line.source_text.text, "What do you need?");
    assert_eq!(
        line.statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Choice]
    );
}

#[test]
fn lowering_summary_stays_stable_for_supported_and_recovered_statements() {
    let source = concat!(
        ":: tavern_arrival default\n",
        "# scene opener\n",
        "> ta_001@be22b6be80ad826f28ab speaker=innkeeper portrait=neutral repeat=true\n",
        "  Welcome.\n",
        "  ? ask_road@88a9cea14cb701044f23\n",
        "    Ask about the road.\n",
        "  Still parent prose.\n",
        "? unsupported_choice@6bc0f66cce756ee22259\n",
        "  Ask about work.\n",
    );

    let lowered = lower(source);

    insta::assert_snapshot!(
        "lowering_summary_stays_stable_for_supported_and_recovered_statements",
        lowered_summary(&lowered)
    );
}
