mod support;

use recite_core::ScalarValue;
use recite_godot::{ReciteDialogueDriver, ReciteOutput};
use recite_runtime::InterpolationValues;

use support::{assert_error_code, assert_line, compile_asset, must_ok, output_kinds};

fn interpolation_asset() -> recite_godot::ReciteDialogueAsset {
    compile_asset(
        "dialogue/interpolation.recite",
        "dialogue/interpolation.recitec",
        concat!(
            ":: start default\n",
            "> greeting@92000000000000000001 bind=(name:string=$name) bind=(ready:bool=$ready)\n",
            "  Hello {name}; ready={ready}.\n",
            "> letters@92000000000000000002 bind=(count:int=$remaining)\n",
            "  You have one letter.\n",
            "  | You have {count} letters.\n",
            "> prompt@92000000000000000003 bind=(name:string=$name)\n",
            "  Pick for {name}.\n",
            "  ? choose@92000000000000000004 bind=(name:string=$name) bind=(ready:bool=$ready)\n",
            "    Choose {name} ({ready}).\n",
            "    -> after\n",
            ":: after\n",
            "> remaining@92000000000000000005 bind=(remaining:int=$remaining)\n",
            "  Remaining: {remaining}.\n",
            "-> END\n",
        ),
    )
}

fn values() -> InterpolationValues {
    let mut values = InterpolationValues::new();
    values.insert("name".to_owned(), ScalarValue::String("Ada".to_owned()));
    values.insert("ready".to_owned(), ScalarValue::Boolean(true));
    values.insert("remaining".to_owned(), ScalarValue::Integer(2));
    values
}

#[test]
fn typed_values_drive_lines_plural_and_choices() {
    let asset = interpolation_asset();
    let mut driver = ReciteDialogueDriver::new();
    driver.set_interpolation_values(values());

    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["line", "line", "prompt"]);
    assert_line(
        &outputs[0],
        "92000000000000000001",
        "Hello Ada; ready=true.",
    );
    assert_line(&outputs[1], "92000000000000000002", "You have 2 letters.");
    let ReciteOutput::Prompt { choices, .. } = &outputs[2] else {
        panic!("expected prompt output, got {:?}", outputs[2]);
    };
    assert_eq!(choices[0].text, "Choose Ada (true).");

    let outputs = must_ok(driver.select_choice("92000000000000000004"));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "92000000000000000005", "Remaining: 2.");
}

#[test]
fn missing_or_wrong_typed_values_project_as_localisation_errors() {
    let asset = interpolation_asset();
    let mut driver = ReciteDialogueDriver::new();
    let mut missing = InterpolationValues::new();
    missing.insert("name".to_owned(), ScalarValue::String("Ada".to_owned()));
    driver.set_interpolation_values(missing);
    assert_error_code(driver.start(&asset, None, None), "localisation_error");

    let mut wrong = InterpolationValues::new();
    wrong.insert("name".to_owned(), ScalarValue::String("Ada".to_owned()));
    wrong.insert("ready".to_owned(), ScalarValue::String("yes".to_owned()));
    driver.set_interpolation_values(wrong);
    assert_error_code(driver.start(&asset, None, None), "localisation_error");
}
