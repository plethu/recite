mod support;

use recite_godot::ReciteDialogueDriver;

use support::{assert_line, compile_asset, must_ok, must_ok_unit, output_kinds};

#[test]
fn snapshots_restore_pending_prompt_without_reemitting_prompt() {
    let asset = compile_asset(
        "dialogue/snapshot.recite",
        "dialogue/snapshot.recitec",
        concat!(
            ":: start default\n",
            "> prompt@88888888888888888881\n",
            "  Save?\n",
            "  ? continue@88888888888888888882\n",
            "    Continue.\n",
            "    -> after\n",
            ":: after\n",
            "> after@88888888888888888883\n",
            "  Restored.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();
    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["prompt"]);

    let snapshot = match driver.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("snapshot should encode: {error}"),
    };
    must_ok_unit(driver.end_session());
    let restored_outputs = must_ok(driver.restore(&asset, &snapshot));
    assert!(restored_outputs.is_empty());

    let outputs = must_ok(driver.select_choice("88888888888888888882"));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "88888888888888888883", "Restored.");
}
