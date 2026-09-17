#![cfg(test)]
use tempfile::TempDir;
mod support;
use support::*;

#[test]
fn private_locale_prefers_its_catalogue_and_falls_back_to_welsh() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let po = "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Base translation\"\n";
    write_file(temp.path(), "locale/cy.po", po);
    write_file(
        temp.path(),
        "locale/cy-x-cofi.po",
        &po.replace("Base translation", "Dialect translation"),
    );
    for (extra, expected) in [
        (
            "cy-x-cofi = [\"locale/cy-x-cofi.po\"]",
            "Dialect translation",
        ),
        ("", "Base translation"),
    ] {
        let fixture = write_file(
            temp.path(),
            "fixture.toml",
            &format!(
                "[dialogue]\nlocale = \"CY-x-COFI\"\n[dialogue.catalogs]\ncy = [\"locale/cy.po\"]\n{extra}\n"
            ),
        );
        let output = run(recite()
            .arg("run")
            .arg(&asset)
            .arg("--block")
            .arg("start")
            .arg("--fixture")
            .arg(&fixture));
        output.assert_success().assert_stderr("");
        output.assert_stdout_contains(expected);
        let trace = run(recite()
            .arg("trace")
            .arg(&asset)
            .arg("--block")
            .arg("start")
            .arg("--fixture")
            .arg(&fixture));
        trace.assert_success().assert_stderr("");
        let value: serde_json::Value = serde_json::from_slice(&trace.stdout).expect("trace JSON");
        assert_eq!(
            value["dialogue_locale_fallbacks"],
            serde_json::json!(["cy-x-cofi", "cy"])
        );
    }
}
