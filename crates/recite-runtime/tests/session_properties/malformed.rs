use recite_runtime::{
    ConditionQuery, ConditionValue, DialogueError, next,
    snapshot::{restore_session, snapshot_session},
    start_scene,
};

use super::fixture;

#[test]
fn malformed_pending_prompt_shapes_fail_before_restoration()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = fixture(2).map_err(|error| error.to_string())?;
    let mut session = start_scene(&asset, None)?;
    let context = |_: ConditionQuery<'_>| Ok(ConditionValue::Bool(true));
    next(&asset, &mut session, &context)?;
    let original = snapshot_session(&session);
    for (operation, detail) in [
        (0, "pending prompt has no choices"),
        (1, "does not match compiled prompt choice count"),
        (
            2,
            "pending prompt choices must match previous prompt choices",
        ),
        (3, "ChoiceId must not be empty"),
        (4, "ended sessions cannot have a pending prompt"),
    ] {
        let mut invalid = original.clone();
        let pending = invalid.pending_prompt.as_mut().expect("fixture prompt");
        match operation {
            0 => pending.choices.clear(),
            1 => {
                pending.choices.pop();
            }
            2 => invalid.previous_prompt_choices.reverse(),
            3 => pending.choices[0].id.clear(),
            _ => invalid.ended = true,
        }
        let Err(DialogueError::InvalidSessionSnapshot { reason, .. }) =
            restore_session(&asset, invalid)
        else {
            panic!("malformed prompt must be refused");
        };
        assert!(reason.contains(detail), "{operation}: {reason}");
    }
    Ok(())
}
