use super::*;

pub(super) const RUNTIME_SURFACE: &str =
    "fixtures/recite/valid/adapter_conformance/runtime_surface.recite";
pub(super) type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn app_with_surface() -> TestResult<(App, bevy_asset::Handle<ReciteDialogueAsset>)> {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(fixture_asset(RUNTIME_SURFACE)?);
    app.update();
    Ok((app, handle))
}

pub(super) fn send(
    app: &mut App,
    cursor: &mut MessageCursor<ReciteOutput>,
    request: ReciteRequest,
) -> Result<ReciteOutput, io::Error> {
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(request);
    app.update();
    cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .last()
        .cloned()
        .ok_or_else(|| io::Error::other("missing adapter output"))
}

pub(super) fn start(
    asset: bevy_asset::Handle<ReciteDialogueAsset>,
    block_id: Option<&str>,
) -> ReciteRequest {
    ReciteRequest::Start {
        asset,
        block_id: block_id.map(str::to_owned),
        locale: None,
        variant: None,
    }
}

pub(super) fn assert_case_error(output: ReciteOutput, id: &str) -> TestResult<()> {
    let case = scenario(id)?;
    let expected = case["steps"]
        .as_array()
        .ok_or_else(|| io::Error::other("missing scenario steps"))?
        .last()
        .ok_or_else(|| io::Error::other("empty scenario steps"))?["expect"]["error_category"]
        .as_str()
        .ok_or_else(|| io::Error::other("missing expected category"))?;
    match output.value {
        ReciteOutputValue::Error { error, .. } => assert_eq!(error.code(), expected, "{id}"),
        other => panic!("{id}: expected error, got {other:?}"),
    }
    Ok(())
}

pub(super) fn register_trusts(app: &mut App, value: bool) {
    app.world_mut()
        .resource_mut::<ReciteConditions>()
        .register("trusts", move |_, _| Ok(ConditionValue::Bool(value)));
}

pub(super) fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

pub(super) fn scenario(id: &str) -> TestResult<Value> {
    let source = fs::read_to_string(workspace_path(
        "fixtures/adapter-conformance/v1/scenarios.json",
    ))?;
    let manifest: Value = serde_json::from_str(&source)?;
    Ok(manifest["scenarios"]
        .as_array()
        .ok_or_else(|| io::Error::other("missing scenarios"))?
        .iter()
        .find(|scenario| scenario["id"] == id)
        .ok_or_else(|| io::Error::other(format!("missing scenario {id}")))?
        .clone())
}

pub(super) fn fixture_asset(fixture: &str) -> TestResult<ReciteDialogueAsset> {
    fixture_asset_with_schema(fixture, SchemaFingerprint::NoSchema)
}

pub(super) fn fixture_asset_with_schema(
    fixture: &str,
    schema: SchemaFingerprint,
) -> TestResult<ReciteDialogueAsset> {
    let source = fs::read_to_string(workspace_path(fixture))?;
    let options = CompileOptions::new(
        CompilerVersion::new("0.1.0")?,
        CompiledAssetId::new("dialogue/conformance.recitec")?,
        SourceMapId::new("conformance-source-map")?,
        schema,
    );
    let report = compile_inputs([CompileInput::new(fixture, &source)], options)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    Ok(ReciteDialogueAsset::from_bytes(
        &report
            .asset
            .ok_or_else(|| io::Error::other("missing compiled fixture"))?
            .messagepack,
    )?)
}
