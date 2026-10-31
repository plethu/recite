mod support;

use recite_godot::ReciteDialogueDriver;
use recite_runtime::ConditionValue;

use support::{assert_error_code, assert_line, compile_asset, must_ok, output_kinds};

#[test]
fn rejects_second_start_while_session_is_active() {
    let asset = compile_asset(
        "dialogue/active.recite",
        "dialogue/active.recitec",
        concat!(
            ":: start default\n",
            "> @qckv0sq\n",
            "  Active?\n",
            "  ? @end\n",
            "    End.\n",
            "    -> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();
    must_ok(driver.start(&asset, None, None));

    assert_error_code(
        driver.start(&asset, None, None),
        "session_already_active_error",
    );
}

#[test]
fn unknown_start_block_uses_contract_error_category() {
    let asset = compile_asset(
        "dialogue/unknown-block.recite",
        "dialogue/unknown-block.recitec",
        concat!(
            ":: start default\n",
            "> @bqyvpqq\n",
            "  Hello.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();

    assert_error_code(
        driver.start(&asset, Some("missing"), None),
        "unknown_start_block_error",
    );
}

#[test]
fn choice_errors_are_structured_and_keep_prompt_pending() {
    let asset = compile_asset(
        "dialogue/choices.recite",
        "dialogue/choices.recitec",
        concat!(
            ":: start default\n",
            "> @b1hymmg\n",
            "  Pick.\n",
            "  ? requires=(trusts(player)) @locked\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? @open\n",
            "    Open.\n",
            "    -> END\n",
            ":: locked\n",
            "> @z0g6c80\n",
            "  Secret.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();
    driver.register_condition("trusts", |_| Ok(ConditionValue::Bool(false)));
    must_ok(driver.start(&asset, None, None));

    assert_error_code(
        driver.select_choice("missing-choice"),
        "invalid_choice_error",
    );
    assert_error_code(driver.select_choice("locked"), "unavailable_choice_error");

    let outputs = must_ok(driver.select_choice("open"));
    assert_eq!(output_kinds(&outputs), ["end"]);
    assert_error_code(driver.select_choice("open"), "stale_choice_error");
}

#[test]
fn changed_asset_is_used_for_next_session_only() {
    let first_asset = compile_asset(
        "dialogue/reload.recite",
        "dialogue/reload.recitec",
        changed_asset_source("Old active asset."),
    );
    let changed_asset = compile_asset(
        "dialogue/reload.recite",
        "dialogue/reload.recitec",
        changed_asset_source("New asset."),
    );
    let mut active_driver = ReciteDialogueDriver::new();
    must_ok(active_driver.start(&first_asset, None, None));

    let outputs = must_ok(active_driver.select_choice("continue"));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "after", "Old active asset.");

    let mut next_driver = ReciteDialogueDriver::new();
    must_ok(next_driver.start(&changed_asset, None, None));
    let outputs = must_ok(next_driver.select_choice("continue"));
    assert_line(&outputs[0], "after", "New asset.");
}

#[test]
fn divert_loop_uses_dialogue_fault_error_category() {
    let asset = compile_asset(
        "dialogue/loop.recite",
        "dialogue/loop.recitec",
        concat!(":: start default\n", "-> start\n",),
    );
    let mut driver = ReciteDialogueDriver::new();

    assert_error_code(driver.start(&asset, None, None), "dialogue_fault_error");
}

fn changed_asset_source(line: &str) -> String {
    format!(
        "{}{}{}",
        concat!(
            ":: start default\n",
            "> @rtdatqr\n",
            "  Reload?\n",
            "  ? @continue\n",
            "    Continue.\n",
            "    -> after\n",
            ":: after\n",
            "> @after\n",
            "  ",
        ),
        line,
        "\n-> END\n"
    )
}
