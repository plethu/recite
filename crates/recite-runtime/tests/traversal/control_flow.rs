use super::*;

#[test]
fn follows_diverts_to_the_target_block_before_emitting() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "-> work\n",
            ":: work\n",
            "> work_line\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "work_line", "Work waits.");
}

#[test]
fn traversal_is_deterministic_for_repeated_sessions() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first\n",
            "  First.\n",
            "-> work\n",
            ":: work\n",
            "> second\n",
            "  Second.\n",
            "-> END\n",
        ),
    );

    let first = run_to_end(&asset);
    let second = run_to_end(&asset);

    assert_eq!(first, second);
}

#[test]
fn internal_divert_loop_returns_traversal_limit_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(":: start default\n", "-> start\n",),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::TraversalLimitExceeded { limit: 10_000 })
    );
}
