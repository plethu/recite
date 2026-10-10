#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn identical_translated_text_and_source_fallback_have_distinct_provenance() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> hello@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        "[dialogue]\nlocale=\"fr-CA\"\n[dialogue.catalogs]\nfr=[\"fr.po\"]\n",
    );
    for translation in ["Hello.", ""] {
        write_file(
            temp.path(),
            "fr.po",
            &format!(
                "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"{translation}\"\n"
            ),
        );
        let output = run(recite()
            .arg("trace")
            .arg(&asset)
            .args(["--block", "start", "--fixture"])
            .arg(&fixture));
        output.assert_success();
        let trace: serde_json::Value = serde_json::from_slice(&output.stdout).expect("trace JSON");
        let line = &trace["events"][0]["line"];
        assert_eq!(line["text"], "Hello.");
        let lookup = &line["localisation"];
        let expected = if translation.is_empty() {
            "missing_entry"
        } else {
            "matched"
        };
        assert_eq!(lookup["outcome"], expected);
        assert_eq!(lookup["attempts"].as_array().expect("attempts").len(), 2);
        for (attempt, (locale, context)) in lookup["attempts"].as_array().unwrap().iter().zip([
            ("fr-CA", "11111111111111111111"),
            ("fr", "11111111111111111111"),
        ]) {
            assert_eq!(attempt["locale"], locale);
            assert_eq!(attempt["context"], context);
        }
        if translation.is_empty() {
            assert!(lookup["matched_locale"].is_null());
        } else {
            assert_eq!(lookup["matched_locale"], "fr");
            assert_eq!(lookup["matched_context"], "11111111111111111111");
            assert_eq!(lookup["attempts"][1]["outcome"], "matched");
        }
    }
}
