use super::*;

#[test]
fn restores_end_state_without_replaying_scene() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "! deferred finished()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    assert_line(next(&asset, &mut session), "start_line", "Start.");
    assert_end_effects(next(&asset, &mut session), ["finished"]);

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores ended state");
    assert_eq!(
        next(&asset, &mut restored),
        Err(DialogueError::SessionEnded)
    );
    assert_effect_functions(restored.deferred_effects(), ["finished"]);
}
