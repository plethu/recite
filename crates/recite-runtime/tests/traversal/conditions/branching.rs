use super::*;

#[test]
fn true_condition_enters_then_branch_and_resumes_parent_range() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> before@a90e86884f3aa3bf3ab7\n",
            "  Before.\n",
            ":if trusts(player, \"hazel rhea\", 3, 0.75, true)\n",
            "  > secret@4dff358182d02d9090e3\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback@4cc0a18fca48337278d1\n",
            "    Fallback.\n",
            "> after@ebb571c54e9fa639e36f\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "a90e86884f3aa3bf3ab7",
        "Before.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "4dff358182d02d9090e3",
        "Secret.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "ebb571c54e9fa639e36f",
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
            "  > secret@0547a337cc70d1ef8296\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback@577a8dcc422289725eee\n",
            "    Fallback.\n",
            "> after@212e132e461f67d686e2\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "577a8dcc422289725eee",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "212e132e461f67d686e2",
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
            "  > secret@5fdf752af69d9fec62ed\n",
            "    Secret.\n",
            "> after@d49fcda74fa96cfd6166\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "d49fcda74fa96cfd6166",
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
            "  > secret@a38565dc5cbb70673ae6\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "a38565dc5cbb70673ae6",
        "Secret.",
    );
}
