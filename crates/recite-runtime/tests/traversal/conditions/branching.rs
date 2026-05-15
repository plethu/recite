use super::*;

#[test]
fn true_condition_enters_then_branch_and_resumes_parent_range() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> before\n",
            "  Before.\n",
            ":if trusts(player, \"hazel rhea\", 3, 0.75, true)\n",
            "  > secret\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback\n",
            "    Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "before",
        "Before.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "secret",
        "Secret.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Ok(empty_end())
    );
    assert_eq!(
        context.calls(),
        [RecordedCall {
            function: "trusts".to_owned(),
            arguments: vec![
                RecordedArgument::Identifier("player".to_owned()),
                RecordedArgument::String("hazel rhea".to_owned()),
                RecordedArgument::Integer(3),
                RecordedArgument::Float(0.75),
                RecordedArgument::Boolean(true),
            ],
        }]
    );
}

#[test]
fn false_condition_enters_else_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback\n",
            "    Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "fallback",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
}

#[test]
fn false_condition_without_else_skips_gated_statements() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
}

#[test]
fn not_condition_inverts_context_result() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if not trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "secret",
        "Secret.",
    );
}
