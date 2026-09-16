//! Run explicitly with --features benchmarks; emits timings and host memory evidence.
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::{Document, ProjectContext, SearchIndex, workload};
use serde_json::{Value, json};
use std::hint::black_box;

fn measure<T>(name: &str, rows: &mut Vec<Value>, operation: impl FnOnce() -> T) -> T {
    let start = WallTime.start();
    let value = operation();
    rows.push(json!({"operation":name, "milliseconds":WallTime.end(start).as_secs_f64()*1000.}));
    value
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let argument = |name: &str| {
        args.iter()
            .position(|value| value == name)
            .and_then(|index| args.get(index + 1))
    };
    let passages = argument("--passages")
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(10_000usize);
    let per_document = argument("--per-document")
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(500usize);
    if passages == 0 || per_document == 0 {
        return Err("positive workload sizes required".into());
    }
    let mut rows = Vec::new();
    let inputs = measure("generate", &mut rows, || {
        workload::project(passages, per_document)
    })?;
    let bytes: usize = inputs.iter().map(|doc| doc.text().len()).sum();
    let index = measure("index", &mut rows, || SearchIndex::build(&inputs));
    assert_eq!(index.len(), passages);
    for _ in 0..5 {
        let (count, _) = measure("search", &mut rows, || {
            index.search(black_box("mara beacon17"), 50)
        });
        assert!(count > 0 || passages <= 17);
        measure("clone_saved_inputs", &mut rows, || {
            black_box(inputs.clone())
        });
    }
    let mut document = measure("open_project_document", &mut rows, || {
        Document::in_project(
            inputs[0].key().clone(),
            inputs[0].text(),
            ProjectContext {
                documents: inputs.clone(),
                schema: None,
            },
        )
    })?;
    let diagnostic_count = document.diagnostics().len();
    assert_eq!(
        diagnostic_count, 0,
        "generated project must validate cleanly"
    );
    let first = measure("project_script", &mut rows, || document.script_snapshot())?;
    for _ in 0..5 {
        let cached = measure("cached_script", &mut rows, || document.script_snapshot())?;
        assert!(std::sync::Arc::ptr_eq(&first, &cached));
    }
    for edit in 0..10 {
        let next = document.source().replacen(
            "remains unanswered",
            &format!("remains unanswered {edit}"),
            1,
        );
        measure("edit", &mut rows, || {
            document.replace_source(document.revision(), next)
        })?;
    }
    for _ in 0..10 {
        assert!(measure("undo", &mut rows, || document.undo())?);
    }
    for _ in 0..10 {
        assert!(measure("redo", &mut rows, || document.redo())?);
    }
    let multiline =
        document
            .source()
            .replacen("The courier waits", "The courier waits\n  and wonders", 1);
    measure("multiline_edit", &mut rows, || {
        document.replace_source(document.revision(), multiline)
    })?;
    assert!(measure("multiline_undo", &mut rows, || document.undo())?);
    let changed_id =
        document
            .source()
            .replacen("@00000000000000000000", "@ffffffffffffffffffff", 1);
    measure("id_edit", &mut rows, || {
        document.replace_source(document.revision(), changed_id)
    })?;
    assert!(measure("id_undo", &mut rows, || document.undo())?);
    let peak = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("VmHWM:"))
                .map(str::to_owned)
        });
    let report = json!({"os":std::env::consts::OS, "arch":std::env::consts::ARCH,
        "profile":"bench", "passages":passages, "documents":inputs.len(), "source_bytes":bytes,
        "diagnostic_count":diagnostic_count, "history_bytes":document.history_bytes(), "process_peak_rss":peak, "samples":rows});
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(path) = argument("--output") {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    Ok(())
}
