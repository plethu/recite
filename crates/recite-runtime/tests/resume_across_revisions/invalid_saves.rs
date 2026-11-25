//! A relocated save is validated like an exact restore before it resumes.

use recite_runtime::snapshot::DialogueEffectModeSnapshot;

use super::*;

fn rejected(asset: &CompiledDialogue, snapshot: recite_runtime::snapshot::DialogueSessionSnapshot) {
    match resume_session(asset, snapshot, &ResumeOptions::new()) {
        Err(DialogueError::InvalidSessionSnapshot { .. }) => {}
        other => panic!("expected an invalid snapshot, got {other:?}"),
    }
}

#[test]
fn relocated_saves_must_ask_for_their_compiled_effects_and_prompt() {
    let base = compile(BASE);
    let text = edited("  Second.\n", "  Second!\n");

    let mut pending = save(&base, SavePoint::PendingGrant);
    pending
        .pending_effect
        .as_mut()
        .expect("pending")
        .request
        .function = "grant_gold".to_owned();
    rejected(&text, pending.clone());
    assert!(
        restore_session(&base, pending).is_err(),
        "exact restore agrees"
    );

    let mut deferred = save(&base, SavePoint::PendingPrompt);
    deferred.deferred_effects[0].mode = DialogueEffectModeSnapshot::Immediate;
    rejected(&text, deferred);

    let mut prompt = save(&base, SavePoint::PendingPrompt);
    prompt.previous_prompt_choices.reverse();
    rejected(&text, prompt);

    let mut empty = save(&base, SavePoint::PendingPrompt);
    empty
        .pending_prompt
        .as_mut()
        .expect("prompt")
        .choices
        .clear();
    empty.previous_prompt_choices.clear();
    rejected(&text, empty);

    // With the block's structure unchanged, the saved choices must be the
    // compiled ones rather than a reason to evaluate the prompt again.
    let mut renamed = save(&base, SavePoint::PendingPrompt);
    for choice in &mut renamed.pending_prompt.as_mut().expect("prompt").choices {
        choice.id = format!("c_bogus_{}", choice.id);
    }
    renamed.previous_prompt_choices = renamed
        .pending_prompt
        .as_ref()
        .expect("prompt")
        .choices
        .iter()
        .map(|choice| choice.id.clone())
        .collect();
    rejected(&text, renamed);
}
