use super::*;
use recite_compiler::compile::{CompileInput, compile_inputs};
use recite_core::project::ProjectManifest;
use tempfile::TempDir;

fn manifest(assets: [&str; 3]) -> ProjectManifestSource {
    let mut text = "format_version = 1\n".to_owned();
    for (i, asset) in assets.iter().enumerate() {
        text.push_str(&format!("[[scenes]]\nid = \"scene.{i}\"\nasset = \"{asset}\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"));
    }
    ProjectManifest::load_str_with_spans("recite.project.toml", &text)
        .source
        .expect("manifest")
}

#[test]
fn shared_missing_and_malformed_assets_keep_each_scene_diagnostic_in_order() {
    let root = TempDir::new().expect("tempdir");
    for malformed in [false, true] {
        if malformed {
            fs::write(root.path().join("z.recitec"), [0xc1]).expect("malformed z");
            fs::write(root.path().join("a.recitec"), [0xc1]).expect("malformed a");
        }
        let manifest = manifest(["z.recitec", "a.recitec", "z.recitec"]);
        let actual =
            validate_project_asset_freshness(root.path(), &manifest, None).expect("diagnostics");
        assert_eq!(actual.len(), 3);
        for (i, diagnostic) in actual.iter().enumerate() {
            assert_eq!(diagnostic.span, manifest.scene_key_span(i, "asset"));
            assert_eq!(
                diagnostic.code.as_str(),
                if malformed {
                    "RECITE_PROJECT007"
                } else {
                    "RECITE_PROJECT003"
                }
            );
            assert!(diagnostic.message.contains(&format!("scene.{i}")));
            assert_eq!(
                diagnostic
                    .presentation
                    .as_ref()
                    .expect("presentation")
                    .arguments()["scene_id"],
                DiagnosticArgumentValue::String(format!("scene.{i}"))
            );
        }
    }
}

#[test]
fn asset_groups_preserve_first_encountered_filesystem_error() {
    let root = TempDir::new().expect("tempdir");
    for name in ["z.recitec", "a.recitec"] {
        fs::create_dir(root.path().join(name)).expect("directory");
    }
    for assets in [
        ["z.recitec", "a.recitec", "z.recitec"],
        ["a.recitec", "z.recitec", "a.recitec"],
    ] {
        let error = validate_project_asset_freshness(root.path(), &manifest(assets), None)
            .expect_err("not files");
        assert!(
            matches!(error, CliError::AssetNotFile { path } if path == root.path().join(assets[0]))
        );
    }
}

#[test]
fn shared_assets_keep_source_freshness_and_scene_specific_checks() {
    let root = TempDir::new().expect("tempdir");
    let source = ":: start default\n> line@11111111111111111111 speaker=hazel\n  Café.\n-> END\n";
    let path = root.path().join("z.recitec");
    let compiled = compile_inputs(
        [CompileInput::new("dialogue.recite", source)],
        crate::fs::compile_options(&path, None).expect("options"),
    )
    .expect("compile")
    .asset
    .expect("asset");
    for name in ["z.recitec", "a.recitec"] {
        fs::write(root.path().join(name), &compiled.messagepack).expect("asset");
    }
    let text = manifest(["z.recitec", "a.recitec", "z.recitec"])
        .source_text()
        .replace(
            "id = \"scene.1\"\nasset = \"a.recitec\"\nblock = \"start\"",
            "id = \"scene.1\"\nasset = \"a.recitec\"\nblock = \"missing\"",
        );
    let manifest = ProjectManifest::load_str_with_spans("recite.project.toml", &text)
        .source
        .expect("manifest");
    for current in [Some(source), Some("changed"), None] {
        let path = root.path().join("dialogue.recite");
        if let Some(current) = current {
            fs::write(&path, current).expect("source");
        } else {
            fs::remove_file(&path).expect("remove");
        }
        let diagnostics = validate_project_asset_freshness(
            root.path(),
            &manifest,
            Some(SchemaFingerprint::NoSchema),
        )
        .expect("freshness");
        let expected = if current == Some(source) {
            vec!["RECITE_PROJECT004"]
        } else {
            vec![
                if current.is_some() {
                    "RECITE_FRESH001"
                } else {
                    "RECITE_PROJECT006"
                },
                "RECITE_PROJECT004",
                if current.is_some() {
                    "RECITE_FRESH001"
                } else {
                    "RECITE_PROJECT006"
                },
                if current.is_some() {
                    "RECITE_FRESH001"
                } else {
                    "RECITE_PROJECT006"
                },
            ]
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
