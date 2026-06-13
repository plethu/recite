mod support;

use recite_godot::ReciteDialogueDriver;

use support::{
    assert_effect, assert_error_code, assert_line, compile_asset, must_ok, output_kinds,
};

#[test]
fn blocking_effects_require_matching_acknowledgement_id() {
    let asset = compile_asset(
        "dialogue/blocking.recite",
        "dialogue/blocking.recitec",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "> after@55555555555555555551\n",
            "  Granted.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();

    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["effect"]);
    let effect_id = assert_effect(&outputs[0], "grant_item", "blocking");

    assert_error_code(
        driver.acknowledge_effect("grant_item#wrong", true, None),
        "effect_acknowledgement_error",
    );

    let outputs = must_ok(driver.acknowledge_effect(&effect_id, false, Some("inventory closed")));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "55555555555555555551", "Granted.");
}
