use super::*;

#[test]
fn condition_failure_is_structured_and_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let failing = RecordingContext::default().failing("trusts", "condition is unavailable");
    let passing = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &failing),
        Err(DialogueError::ConditionEvaluationFailed {
            function: "trusts".to_owned(),
            reason: "condition is unavailable".to_owned(),
        })
    );
    assert_line(
        next_with_context(&asset, &mut session, &passing),
        "secret",
        "Secret.",
    );
}

#[test]
fn deeply_nested_condition_returns_structured_depth_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let CompiledStatementKind::If { condition, .. } = &mut asset.statements[0].kind else {
        panic!("expected if statement");
    };
    *condition = deeply_nested_condition(150);
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::ConditionDepthLimitExceeded { limit: 128 })
    );
    assert!(
        context.calls().is_empty(),
        "runtime should stop before reaching the deeply nested call"
    );
}

#[test]
fn boolean_conditions_short_circuit_left_to_right() {
    let and_asset = compile_asset(
        "dialogue/and.recite",
        concat!(
            ":: start default\n",
            ":if first() and missing()\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let and_context = RecordingContext::default().with("first", false);
    let mut and_session = start_scene(&and_asset, None).expect("starts");

    assert_line(
        next_with_context(&and_asset, &mut and_session, &and_context),
        "after",
        "After.",
    );
    assert_eq!(
        and_context.calls(),
        [RecordedCall {
            function: "first".to_owned(),
            arguments: Vec::new(),
        }]
    );

    let or_asset = compile_asset(
        "dialogue/or.recite",
        concat!(
            ":: start default\n",
            ":if first() or missing()\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let or_context = RecordingContext::default().with("first", true);
    let mut or_session = start_scene(&or_asset, None).expect("starts");

    assert_line(
        next_with_context(&or_asset, &mut or_session, &or_context),
        "secret",
        "Secret.",
    );
    assert_eq!(
        or_context.calls(),
        [RecordedCall {
            function: "first".to_owned(),
            arguments: Vec::new(),
        }]
    );
}
