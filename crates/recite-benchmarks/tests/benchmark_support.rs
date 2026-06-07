use recite_benchmarks::catalog::{CatalogProvider, parse_po_catalog};
use recite_benchmarks::compiler::CompilerProject;
use recite_benchmarks::fixture_context::RuntimeFixture;
use recite_benchmarks::id_metrics::{
    compiled_id_metrics, id_storage_report, runtime_fixture_id_metrics, source_id_metrics,
};
use recite_benchmarks::lsp::LspBenchmarkProject;
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::scale::{parse_fixture_list, parse_scale_list};
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale, compiler};
use std::fs;
use std::sync::Mutex;

static GENERATED_PROJECT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn scale_selection_defaults_are_tiny_and_small() {
    assert_eq!(
        BenchmarkScale::DEFAULT,
        [BenchmarkScale::Tiny, BenchmarkScale::Small]
    );
}

#[test]
fn explicit_full_scale_selection_preserves_order() -> Result<(), Box<dyn std::error::Error>> {
    let scales = parse_scale_list("tiny,small,medium,large,epic")?;
    assert_eq!(scales, BenchmarkScale::ALL);
    Ok(())
}

#[test]
fn scale_selection_rejects_empty_entries() {
    assert!(parse_scale_list("tiny,,small").is_err());
}

#[test]
fn fixture_selection_supports_synthetic_scales_and_realistic_pack()
-> Result<(), Box<dyn std::error::Error>> {
    let fixtures = parse_fixture_list("tiny, realistic:v1-pack, tiny")?;
    assert_eq!(
        fixtures,
        [
            BenchmarkFixture::Synthetic(BenchmarkScale::Tiny),
            BenchmarkFixture::RealisticV1Pack,
        ]
    );
    assert_eq!(
        BenchmarkFixture::DEFAULT.len(),
        BenchmarkScale::DEFAULT.len()
    );
    Ok(())
}

#[test]
fn tiny_project_loads_and_matches_checked_summary() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    assert_eq!(project.summary().profile.name, "tiny");
    assert_eq!(project.source_files()?.len(), 2);
    Ok(())
}

#[test]
fn realistic_v1_pack_loads_and_matches_checked_summary() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load_fixture(BenchmarkFixture::RealisticV1Pack)?;
    let summary = project
        .realistic_summary()
        .expect("realistic project exposes realistic summary");

    assert_eq!(project.fixture_label(), "realistic:v1-pack");
    assert_eq!(summary.name, "v1-pack");
    assert_eq!(summary.counts.source_files, 5);
    assert_eq!(summary.counts.choices, 12);
    assert_eq!(project.source_files()?.len(), 5);
    Ok(())
}

#[test]
fn realistic_v1_pack_compiles_extracts_catalog_and_traverses()
-> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load_fixture(BenchmarkFixture::RealisticV1Pack)?;
    let runtime_fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
    let catalog = CatalogProvider::load(&project, &runtime_fixture)?;
    assert_eq!(
        catalog.get("1111111111111111111b"),
        Some("Clock notice: Meridian Station reports minute zero before the embassy relay.")
    );

    let compiler = CompilerProject::load(&project)?;
    assert!(compiler::validate_with_schema(&compiler.source_files(), compiler.schema()).is_ok());
    let pot = compiler::extract_pot(&compiler)?;
    assert!(pot.entries.len() >= 20);

    let compiled = compiler.compile_with_schema()?;
    assert_eq!(compiled.asset().dialogue.sources.len(), 5);
    assert!(compiled.asset().dialogue.effects.len() >= 6);

    let runtime = RuntimeProject::load(&project, &compiled)?;
    let driver = runtime.driver();

    let mut line_session = driver.session_before_first_line()?;
    let _line = driver.next_line(&mut line_session)?;

    let mut localised_session = driver.localised_session_before_first_line()?;
    let _localised_line = driver.localised_next(&mut localised_session)?;

    let mut prompt_session = driver.session_before_first_prompt()?;
    let _prompt = driver.next_prompt(&mut prompt_session)?;

    let mut deferred_session = driver.session_before_deferred_effect()?;
    let _deferred = driver.deferred_effect(&mut deferred_session)?;

    let first = runtime.driver().full_traversal()?;
    let second = runtime.driver().full_traversal()?;
    assert_eq!(first, second);
    assert!(first >= 10);
    Ok(())
}

#[test]
fn tiny_lsp_benchmark_probes_are_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let first = lsp.probes();
    let second = lsp.probes();

    assert_eq!(first.document.project_relative_path, "shard-000.recite");
    assert_eq!(
        first.document.project_relative_path,
        second.document.project_relative_path
    );
    assert_eq!(first.completion.position, second.completion.position);
    assert_eq!(first.definition.position, second.definition.position);
    assert_eq!(first.rename.position, second.rename.position);
    Ok(())
}

#[test]
fn tiny_lsp_initial_index_reports_stable_memory_counts() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let report = lsp.memory_report();

    assert_eq!(report.source_files, 2);
    assert!(report.indexed_source_bytes > 0);
    assert_eq!(report.block_definitions, 10);
    assert!(report.block_references > 0);
    assert!(report.line_ids > 0);
    assert!(report.choice_ids > 0);
    assert!(report.estimated_summary_bytes >= report.indexed_source_bytes);
    assert!(report.to_markdown().contains("estimated_summary_bytes"));
    Ok(())
}

#[test]
fn tiny_lsp_driver_exercises_editor_operations() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let mut driver = lsp.driver();
    let probes = driver.probes();

    let _diagnostics = driver.open_file(&probes.document);
    let _changed = driver.change_file(&probes.document);
    let _refreshed = driver.diagnostics_refresh(&probes.document);
    assert!(driver.completion(&probes.completion).is_some());
    assert!(driver.definition(&probes.definition).is_some());
    assert!(driver.rename(&probes.rename, "renamed_block").is_some());
    Ok(())
}

#[test]
fn tiny_lsp_stale_change_does_not_advance_generation() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let mut driver = lsp.driver();
    let probes = driver.probes();

    assert!(driver.stale_change_is_suppressed(&probes.document));
    Ok(())
}

#[test]
fn generated_small_project_matches_checked_summary() -> Result<(), Box<dyn std::error::Error>> {
    let _generated_project_guard = GENERATED_PROJECT_LOCK
        .lock()
        .expect("lock generated project");
    let project = BenchmarkProject::load(BenchmarkScale::Small)?;
    assert_eq!(project.summary().profile.name, "small");
    assert!(
        project
            .root()
            .ends_with("target/recite-benchmarks/generated/small")
    );
    Ok(())
}

#[test]
fn generated_project_replaces_stale_output() -> Result<(), Box<dyn std::error::Error>> {
    let _generated_project_guard = GENERATED_PROJECT_LOCK
        .lock()
        .expect("lock generated project");
    let project = BenchmarkProject::load(BenchmarkScale::Small)?;
    let stale = project.root().join("src/stale.recite");
    fs::write(&stale, ":: stale\n> stale\n")?;

    let project = BenchmarkProject::load(BenchmarkScale::Small)?;

    assert!(!stale.exists());
    assert!(
        project
            .source_files()?
            .iter()
            .all(|file| file.path != "src/stale.recite")
    );
    Ok(())
}

#[test]
fn catalog_lookup_uses_msgctxt_as_stable_id() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = parse_po_catalog(
        r#"
msgctxt "ab4a03e66ca89a7d2f10"
msgid "source text"
msgstr "translated text"
"#,
    )?;
    assert_eq!(
        catalog.get("ab4a03e66ca89a7d2f10").map(String::as_str),
        Some("translated text")
    );
    Ok(())
}

#[test]
fn tiny_catalog_loads_from_runtime_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
    let catalog = CatalogProvider::load(&project, &fixture)?;
    assert_eq!(
        catalog.get("44bbd8153af5a8182d93"),
        Some("line translation for 44bbd8153af5a8182d93")
    );
    Ok(())
}

#[test]
fn tiny_compiler_smoke_builds_asset() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let compiler = CompilerProject::load(&project)?;
    assert!(compiler::validate_with_schema(&compiler.source_files(), compiler.schema()).is_ok());
    let compiled = compiler.compile_with_schema()?;
    assert_eq!(compiled.asset().dialogue.blocks.len(), 10);
    Ok(())
}

#[test]
fn tiny_id_metrics_cover_source_compiled_and_runtime_fixture_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let compiler = CompilerProject::load(&project)?;
    let source_ids = source_id_metrics(&compiler.source_files());
    let compiled = compiler.compile_with_schema()?;
    let compiled_ids = compiled_id_metrics(&compiled.asset().dialogue);
    let runtime_fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
    let runtime_ids = runtime_fixture_id_metrics(&runtime_fixture);

    assert!(source_ids.total.count >= 130);
    assert!(compiled_ids.total.count >= source_ids.total.count);
    assert!(runtime_ids.total.count > 1);
    assert!(
        compiled_ids.total.compact_heap_payload_bytes
            < compiled_ids.total.string_heap_payload_bytes
    );
    Ok(())
}

#[test]
fn id_storage_report_keeps_id_wrappers_string_sized() {
    let report = id_storage_report();

    assert_eq!(report.id_size_bytes, report.string_size_bytes);
    assert_eq!(
        report.compact_inline_capacity_bytes,
        report.string_size_bytes
    );
}

#[test]
fn tiny_runtime_smoke_traverses_default_scene() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    let compiler = CompilerProject::load(&project)?;
    let compiled = compiler.compile_with_schema()?;
    let runtime = RuntimeProject::load(&project, &compiled)?;
    let events = runtime.driver().full_traversal()?;
    assert!(events > 10);
    Ok(())
}
