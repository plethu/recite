use super::*;
use recite_core::compiled::CompiledArgument;

#[test]
fn next_rejects_changed_executable_tables_with_unchanged_metadata() {
    let original = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred grant_item(original_item)\n",
            "> prompt@12345678901234567890\n",
            "  Prompt.\n",
            "  ? go@12345678901234567891\n",
            "    Go.\n",
            "    -> END\n",
        ),
    );
    let session = start_scene(&original, None).expect("starts");

    let mut changed_line = original.clone().into_payload();
    changed_line.lines[0].source_text = "Changed prompt.".to_owned();
    changed_line.lines[0].authored_source_text = "Changed prompt.".to_owned();
    let changed_line = CompiledDialogue::new(changed_line);
    let mut changed_choice = original.clone().into_payload();
    changed_choice.choices[0].source_text = "Changed choice.".to_owned();
    changed_choice.choices[0].authored_source_text = "Changed choice.".to_owned();
    let changed_choice = CompiledDialogue::new(changed_choice);
    let mut changed_effect = original.clone().into_payload();
    changed_effect.effects[0].args[0] = CompiledArgument::Identifier("changed_item".to_owned());
    let changed_effect = CompiledDialogue::new(changed_effect);

    for candidate in [&changed_line, &changed_choice, &changed_effect] {
        assert_eq!(candidate.header, original.header);
        assert_eq!(candidate.sources, original.sources);
        let mut attempt = session.clone();
        assert!(matches!(
            runtime_next(candidate, &mut attempt, &EmptyDialogueContext),
            Err(DialogueError::AssetContentMismatch { reason, .. })
                if reason.contains("compiled payload fingerprint")
        ));
    }

    let mut equal_asset_session = session;
    runtime_next(
        &original.clone(),
        &mut equal_asset_session,
        &EmptyDialogueContext,
    )
    .expect("equal prepared content remains valid");
}

#[test]
fn choose_rejects_a_changed_choice_before_advancing() {
    let original = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt@12345678901234567890\n",
            "  Prompt.\n",
            "  ? go@12345678901234567891\n",
            "    Go.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&original, None).expect("starts");
    runtime_next(&original, &mut session, &EmptyDialogueContext).expect("prompt");
    let choice_id = original.choices[0].id.clone();
    let mut changed = original.clone().into_payload();
    changed.choices[0].source_text = "Changed choice.".to_owned();
    changed.choices[0].authored_source_text = "Changed choice.".to_owned();
    let changed = CompiledDialogue::new(changed);

    assert!(matches!(
        runtime_choose(&changed, &mut session, choice_id, &EmptyDialogueContext),
        Err(DialogueError::AssetContentMismatch { .. })
    ));
}
