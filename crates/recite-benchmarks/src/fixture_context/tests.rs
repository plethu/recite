use recite_runtime::ConditionArgument;

use super::{RuntimeFixture, format_argument};

#[test]
fn loads_fixture_values() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = RuntimeFixture::load(
        r#"
        [dialogue]
        locale = "en-US"
        [dialogue.catalogs]
        en-US = ["locales/en-US.po"]
        [conditions]
        "flag(\"flag_00\")" = true
        "relationship(speaker_00, speaker_01)" = { enum = "active" }
        [choices]
        line_00000_000 = "choice_00000_000"
        [effects]
        auto_ack_blocking = true
        "#,
    )?;

    assert_eq!(fixture.locale().as_str(), "en-US");
    assert_eq!(
        fixture.choice_for_line("line_00000_000")?.as_str(),
        "choice_00000_000"
    );
    assert!(fixture.auto_ack_blocking());
    Ok(())
}

#[test]
fn builds_condition_keys_from_typed_arguments() {
    assert_eq!(
        format_argument(ConditionArgument::Identifier("speaker_00")),
        "speaker_00"
    );
    assert_eq!(
        format_argument(ConditionArgument::String("flag_00")),
        "\"flag_00\""
    );
}

#[test]
fn exposes_expected_type_names_for_context_errors() {
    assert_eq!(
        format!("{:?}", recite_runtime::ConditionExpectedType::Bool),
        "Bool"
    );
}
