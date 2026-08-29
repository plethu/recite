#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn runtime_delivers_lossless_translated_inline_markup() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue-markup.recite",
        concat!(
            ":: start default\n",
            "> intro@11111111111111111111\n",
            "  [slow]Hello [em]world[/em][/slow]\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue-markup.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        concat!(
            "msgctxt \"11111111111111111111\"\n",
            "msgid \"[slow]Hello [em]world[/em][/slow]\"\n",
            "msgstr \"[slow]Monde [em]bonjour[/em][/slow]\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr"

[dialogue.catalogs]
fr = ["locale/fr.po"]
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
    output.assert_stdout_contains("line 11111111111111111111: [slow]Monde [em]bonjour[/em][/slow]");
}
