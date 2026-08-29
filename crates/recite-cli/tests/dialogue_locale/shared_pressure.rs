use super::*;

#[test]
fn shared_language_pressure_fixture_exercises_locale_fallback_and_interpolation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "src/language_pressure.recite",
        include_str!("../../../../fixtures/recite/valid/language_pressure.recite"),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr-CA.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr-CA\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
            "\n",
            "msgctxt \"55667788990011223344\"\n",
            "msgid \"[slow]The tide is turning, {traveller_name}.[/slow]\"\n",
            "msgstr \"[slow]La marée tourne, {traveller_name}.[/slow]\"\n",
            "\n",
            "msgctxt \"66778899001122334455\"\n",
            "msgid \"Tell me about {topic}.\"\n",
            "msgstr \"Parle-moi de {topic}.\"\n",
        ),
    );
    write_file(
        temp.path(),
        "locale/fr.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n",
            "\n",
            "msgctxt \"0a1b2c3d4e5f60718293\"\n",
            "msgid \"You have one letter.\"\n",
            "msgid_plural \"You have {count} letters.\"\n",
            "msgstr[0] \"Vous avez une lettre.\"\n",
            "msgstr[1] \"Vous avez {count} lettres.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr-CA"

[dialogue.catalogs]
"fr-CA" = ["locale/fr-CA.po"]
fr = ["locale/fr.po"]

[choices]
"55667788990011223344" = "66778899001122334455"

[interpolation_values]
traveller_name = { string = "Mara" }
topic = { string = "la marée" }
letters_remaining = { int = 2 }
"#,
    );

    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("marche.default")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    let trace: serde_json::Value = serde_json::from_slice(&output.stdout).expect("trace is JSON");
    assert_eq!(trace["dialogue_locale"], "fr-CA");
    assert_eq!(
        trace["dialogue_locale_fallbacks"],
        serde_json::json!(["fr-CA", "fr"])
    );

    let events = trace["events"].as_array().expect("events array");
    let prompt = events
        .iter()
        .find(|event| event["type"] == "prompt")
        .expect("prompt event");
    assert_eq!(
        prompt["prompt"]["line"]["text"],
        "[slow]La marée tourne, Mara.[/slow]"
    );
    assert_eq!(
        prompt["prompt"]["choices"][0]["text"],
        "Parle-moi de la marée."
    );

    let plural = events
        .iter()
        .find(|event| event["line"]["id"] == "0a1b2c3d4e5f60718293")
        .expect("plural line event");
    assert_eq!(plural["line"]["text"], "Vous avez 2 lettres.");
    assert_eq!(plural["line"]["plural"]["count"], 2);
    assert_eq!(plural["line"]["plural"]["matched_locale"], "fr");
    assert_eq!(plural["line"]["plural"]["attempts"][0]["locale"], "fr-CA");
    assert_eq!(
        plural["line"]["plural"]["attempts"][0]["outcome"],
        "missing_entry"
    );

    let fallback = events
        .iter()
        .find(|event| event["line"]["id"] == "77889900112233445566")
        .expect("source fallback line event");
    assert_eq!(fallback["line"]["text"], "The ledger closes.");
}
