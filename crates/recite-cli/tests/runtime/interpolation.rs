use tempfile::TempDir;

use super::support::*;

#[test]
fn run_fixture_supplies_explicitly_tagged_interpolation_values() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@11111111111111111111 bind=(name:string=$display) bind=(count:int=$remaining)\n",
            "  Hello {name}, you have {count}.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[interpolation_values]
display = { string = "Ada" }
remaining = { int = 3 }
"#,
    );
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("line 11111111111111111111: Hello Ada, you have 3.");
}

#[test]
fn run_fixture_only_resolves_bindings_in_the_selected_plural_form() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> letters@11111111111111111111 bind=(count:int=$remaining) bind=(name:string=$name)\n",
            "  {name} has one letter.\n",
            "  | You have {count} letters.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let plural_fixture = write_file(
        temp.path(),
        "plural.toml",
        r#"[interpolation_values]
remaining = { int = 2 }
"#,
    );
    let plural_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&plural_fixture));
    plural_output.assert_success().assert_stderr("");
    plural_output.assert_stdout_contains("line 11111111111111111111: You have 2 letters.");

    let singular_fixture = write_file(
        temp.path(),
        "singular.toml",
        r#"[interpolation_values]
remaining = { int = 1 }
"#,
    );
    let singular_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&singular_fixture));
    assert_diagnostic_failure(&singular_output);
    singular_output.assert_stderr_contains("missing interpolation value `name`");
}

#[test]
fn trace_exposes_structured_choice_availability_reasons() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@82db0b1dab0a52136d77\n",
            "  Welcome.\n",
            "  ? ask_news@e8572a78baac6863754d requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "    Ask for private news.\n",
            "    -> END\n",
            "  ? leave@be22df697e7ee4d7ba1b\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    write_file(
        temp.path(),
        "locale/fr-FR.po",
        concat!(
            "msgctxt \"availability_reason:innkeeper_trust_hint\"\n",
            "msgid \"The innkeeper is not ready to share that.\"\n",
            "msgstr \"L'aubergiste n'est pas prête à partager cela.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr-FR"

[dialogue.catalogs]
"fr-FR" = ["locale/fr-FR.po"]

[conditions]
"trust_gte(hazel, rhea, 3)" = false

[choices]
82db0b1dab0a52136d77 = "be22df697e7ee4d7ba1b"
"#,
    );

    let trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    trace_output.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&trace_output.stdout).expect("trace is JSON");
    let choices = trace["events"][1]["prompt"]["choices"]
        .as_array()
        .expect("prompt choices");
    let ask_news = choices
        .iter()
        .find(|choice| choice["id"] == "e8572a78baac6863754d")
        .expect("ask_news choice");

    assert_eq!(ask_news["is_available"], false);
    assert_eq!(
        ask_news["unavailable_reason"],
        "L'aubergiste n'est pas prête à partager cela."
    );
    assert_eq!(
        ask_news["availability"]["primary_reason"]["localized_template"],
        "L'aubergiste n'est pas prête à partager cela."
    );
    assert_eq!(ask_news["availability"]["is_available"], false);
    assert_eq!(
        ask_news["availability"]["primary_reason"]["origin"],
        serde_json::json!({
            "type": "requirement_expression",
            "source_text": "requires=(trust_gte(hazel, rhea, 3))"
        })
    );
    assert_eq!(
        ask_news["availability"]["reason_tree"],
        serde_json::json!({
            "type": "reason",
            "value": {
                "id": "trust_too_low",
                "source_text": "{subject} does not trust {target} enough ({threshold}).",
                "localized_template": "{subject} does not trust {target} enough ({threshold}).",
                "text": "hazel does not trust rhea enough (3).",
                "origin": {
                    "type": "condition_call",
                    "function": "trust_gte",
                    "args": [
                        { "type": "identifier", "value": "hazel" },
                        { "type": "identifier", "value": "rhea" },
                        { "type": "integer", "value": 3 }
                    ]
                },
                "args": [
                    {
                        "name": "subject",
                        "value": { "type": "identifier", "value": "hazel" }
                    },
                    {
                        "name": "target",
                        "value": { "type": "identifier", "value": "rhea" }
                    },
                    {
                        "name": "threshold",
                        "value": { "type": "integer", "value": 3 }
                    }
                ]
            }
        })
    );
}
