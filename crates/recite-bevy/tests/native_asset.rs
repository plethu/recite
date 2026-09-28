use std::fs;
use std::time::Duration;

use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::{AssetPlugin, AssetServer, Assets};
use bevy_ecs::message::{MessageCursor, Messages};
use recite_bevy::{
    AdapterErrorKind, ReciteAssetImport, ReciteAssetStatus, ReciteDialogueAsset, ReciteOwner,
    RecitePlugin, ReciteRequest,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use tempfile::TempDir;

fn compile(source: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("0.1.0")?,
        CompiledAssetId::new("dialogue/native.recitec")?,
        SourceMapId::new("native-source-map")?,
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs([CompileInput::new("native.recite", source)], options)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    Ok(report
        .asset
        .ok_or_else(|| std::io::Error::other("missing asset"))?
        .messagepack)
}

#[test]
fn asset_server_loads_compiled_bytes_and_rejects_bad_refresh_with_retained_revision()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let path = temp.path().join("dialogue.recitec");
    fs::write(
        &path,
        compile(":: start default\n> first@11111111111111111111\n  First.\n-> END\n")?,
    )?;
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        AssetPlugin {
            file_path: temp.path().to_string_lossy().into_owned(),
            ..Default::default()
        },
        RecitePlugin,
    ));
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<ReciteDialogueAsset>("dialogue.recitec");
    let mut cursor = MessageCursor::<ReciteAssetStatus>::default();
    let mut accepted = None;
    for _ in 0..400 {
        app.update();
        let statuses = cursor
            .read(app.world().resource::<Messages<ReciteAssetStatus>>())
            .cloned()
            .collect::<Vec<_>>();
        if let Some(revision) = statuses.iter().find_map(|status| match &status.import {
            ReciteAssetImport::Accepted { revision } => Some(revision.clone()),
            _ => None,
        }) {
            accepted = Some(revision);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let accepted = accepted
        .ok_or_else(|| std::io::Error::other("asset server did not load compiled asset"))?;
    assert!(
        app.world()
            .resource::<Assets<ReciteDialogueAsset>>()
            .get(&handle)
            .is_some()
    );
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(ReciteRequest::Start {
            asset: handle.clone(),
            block_id: None,
            locale: None,
            variant: None,
        });
    app.update();
    assert_eq!(
        app.world().resource::<ReciteOwner>().active_revision(),
        Some(&accepted)
    );

    fs::write(&path, b"not a recite asset")?;
    app.world()
        .resource::<AssetServer>()
        .reload("dialogue.recitec");
    let mut rejected = None;
    for _ in 0..400 {
        app.update();
        let statuses = cursor
            .read(app.world().resource::<Messages<ReciteAssetStatus>>())
            .cloned()
            .collect::<Vec<_>>();
        if let Some(status) = statuses
            .into_iter()
            .find(|status| matches!(status.import, ReciteAssetImport::Rejected { .. }))
        {
            rejected = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let rejected =
        rejected.ok_or_else(|| std::io::Error::other("rejected refresh not reported"))?;
    assert_eq!(rejected.path.as_deref(), Some("dialogue.recitec"));
    assert!(
        matches!(rejected.import, ReciteAssetImport::Rejected { error, retained: Some(revision) }
        if error.kind() == AdapterErrorKind::AssetLoadOrDecode && revision == accepted)
    );
    assert_eq!(
        app.world().resource::<ReciteOwner>().active_revision(),
        Some(&accepted)
    );
    Ok(())
}
