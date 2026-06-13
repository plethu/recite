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
            "> prompt@33333333333333333331\n",
            "  Active?\n",
            "  ? end@33333333333333333332\n",
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
            "> intro@33333333333333333333\n",
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
            "> prompt@44444444444444444441\n",
            "  Pick.\n",
            "  ? locked@44444444444444444442 requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? open@44444444444444444443\n",
            "    Open.\n",
            "    -> END\n",
            ":: locked\n",
            "> locked_line@44444444444444444444\n",
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
    assert_error_code(
        driver.select_choice("44444444444444444442"),
        "unavailable_choice_error",
    );

    let outputs = must_ok(driver.select_choice("44444444444444444443"));
    assert_eq!(output_kinds(&outputs), ["end"]);
    assert_error_code(
        driver.select_choice("44444444444444444443"),
        "stale_choice_error",
    );
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

    let outputs = must_ok(active_driver.select_choice("99999999999999999992"));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "99999999999999999993", "Old active asset.");

    let mut next_driver = ReciteDialogueDriver::new();
    must_ok(next_driver.start(&changed_asset, None, None));
    let outputs = must_ok(next_driver.select_choice("99999999999999999992"));
    assert_line(&outputs[0], "99999999999999999993", "New asset.");
}

fn changed_asset_source(line: &str) -> String {
    format!(
        "{}{}{}",
        concat!(
            ":: start default\n",
            "> prompt@99999999999999999991\n",
            "  Reload?\n",
            "  ? continue@99999999999999999992\n",
            "    Continue.\n",
            "    -> after\n",
            ":: after\n",
            "> after@99999999999999999993\n",
            "  ",
        ),
        line,
        "\n-> END\n"
    )
}
