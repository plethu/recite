use recite_config::ProducerRegistration;
const REGISTRATION: &str = r#"
version = 1
[producer]
kind = "bevy"
id = "dialogue"
[generate]
program = "cargo"
args = ["run", "--bin", "schema", "--", "{output}"]
[editor]
program = "code"
args = ["--goto", "{file}:{line}:{column}"]
[[sources]]
kind = "condition"
name = "has_item"
file = "game/dialogue.rs"
line = 12
"#;
#[test]
fn registration_retains_argv_identity_and_source_coordinates() {
    let registration = ProducerRegistration::parse(REGISTRATION).unwrap();
    assert_eq!(registration.producer().id(), "dialogue");
    assert_eq!(registration.generate().args()[4], "{output}");
    assert_eq!(registration.sources()[0].line(), 12);
    assert_eq!(registration.sources()[0].column(), 1);
}
#[test]
fn rejects_unsupported_versions_ambiguous_sources_and_invalid_placeholders() {
    for invalid in [
        REGISTRATION.replace("version = 1", "version = 2"),
        REGISTRATION.replace("{output}", "{unknown}"),
        REGISTRATION.replace("line = 12", "line = 0"),
        REGISTRATION.replace("game/dialogue.rs", "../dialogue.rs"),
        format!(
            "{REGISTRATION}\n[[sources]]\nkind=\"condition\"\nname=\"has_item\"\nfile=\"other.rs\"\nline=1\n"
        ),
    ] {
        assert!(ProducerRegistration::parse(&invalid).is_err());
    }
}
