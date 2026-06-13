mod support;

use std::cell::RefCell;
use std::rc::Rc;

use recite_godot::{AdapterValue, ReciteDialogueDriver};
use recite_runtime::ConditionValue;

use support::{assert_error_code, assert_line, compile_asset, must_ok, output_kinds};

#[test]
fn condition_handlers_receive_structured_arguments() {
    let asset = compile_asset(
        "dialogue/conditions.recite",
        "dialogue/conditions.recitec",
        concat!(
            ":: start default\n",
            ":if trusts(player, \"hazel\", 3, true)\n",
            "  > trusted@66666666666666666661\n",
            "    Trusted.\n",
            "-> END\n",
        ),
    );
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded_calls = Rc::clone(&calls);
    let mut driver = ReciteDialogueDriver::new();
    driver.register_condition("trusts", move |call| {
        recorded_calls.borrow_mut().push((
            call.function().to_owned(),
            call.arguments().collect::<Vec<_>>(),
        ));
        Ok(ConditionValue::Bool(true))
    });

    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "66666666666666666661", "Trusted.");
    assert_eq!(
        calls.borrow().as_slice(),
        [(
            "trusts".to_owned(),
            vec![
                AdapterValue::Identifier("player".to_owned()),
                AdapterValue::String("hazel".to_owned()),
                AdapterValue::Integer(3),
                AdapterValue::Boolean(true),
            ],
        )]
    );
}

#[test]
fn missing_condition_handler_uses_stable_adapter_error() {
    let asset = compile_asset(
        "dialogue/missing-condition.recite",
        "dialogue/missing-condition.recitec",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > trusted@77777777777777777771\n",
            "    Trusted.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();

    assert_error_code(
        driver.start(&asset, None, None),
        "missing_condition_handler_error",
    );
}

#[test]
fn later_condition_error_rolls_back_outputs_emitted_by_choice() {
    let asset = compile_asset(
        "dialogue/choice-condition-rollback.recite",
        "dialogue/choice-condition-rollback.recitec",
        concat!(
            ":: start default\n",
            "> prompt@77777777777777777772\n",
            "  Enter?\n",
            "  ? enter@77777777777777777773\n",
            "    Enter.\n",
            "    -> guarded\n",
            ":: guarded\n",
            "> before@77777777777777777774\n",
            "  Before condition.\n",
            ":if can_continue(player)\n",
            "  > after@77777777777777777775\n",
            "    After condition.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();
    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["prompt"]);

    assert_error_code(
        driver.select_choice("77777777777777777773"),
        "missing_condition_handler_error",
    );

    driver.register_condition("can_continue", |_| Ok(ConditionValue::Bool(true)));
    let outputs = must_ok(driver.select_choice("77777777777777777773"));
    assert_eq!(output_kinds(&outputs), ["line", "line", "end"]);
    assert_line(&outputs[0], "77777777777777777774", "Before condition.");
    assert_line(&outputs[1], "77777777777777777775", "After condition.");
}
