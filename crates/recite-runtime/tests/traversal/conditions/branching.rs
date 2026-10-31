use super::*;

#[test]
fn true_condition_enters_then_branch_and_resumes_parent_range() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @before\n",
            "  Before.\n",
            ":if trusts(player, \"hazel rhea\", 3, 0.75, true)\n",
            "  > @r2res3q\n",
            "    Secret.\n",
            ":else\n",
            "  > @bfm3at4\n",
            "    Fallback.\n",
            "> @g201j3j\n",
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
        "r2res3q",
        "Secret.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "g201j3j",
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
            "  > @tysehr8\n",
            "    Secret.\n",
            ":else\n",
            "  > @rw5t851\n",
            "    Fallback.\n",
            "> @dazzq65\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "rw5t851",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "dazzq65",
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
            "  > @ffwjkvq\n",
            "    Secret.\n",
            "> @sjqnnx0\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "sjqnnx0",
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
            "  > @vpn3etk\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "vpn3etk",
        "Secret.",
    );
}
