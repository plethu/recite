use recite_benchmarks::catalog::{CatalogProvider, parse_po_catalog};
use recite_benchmarks::compiler::CompilerProject;
use recite_benchmarks::fixture_context::RuntimeFixture;
use recite_benchmarks::id_metrics::{
    compiled_id_metrics, id_storage_report, runtime_fixture_id_metrics, source_id_metrics,
};
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::scale::parse_scale_list;
use recite_benchmarks::{BenchmarkScale, compiler};
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
fn tiny_project_loads_and_matches_checked_summary() -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(BenchmarkScale::Tiny)?;
    assert_eq!(project.summary().profile.name, "tiny");
    assert_eq!(project.source_files()?.len(), 2);
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
msgctxt "line_00000_000"
msgid "source text"
msgstr "translated text"
"#,
    )?;
    assert_eq!(
        catalog.get("line_00000_000").map(String::as_str),
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
        catalog.get("line_00000_000"),
        Some("line translation for line_00000_000")
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
