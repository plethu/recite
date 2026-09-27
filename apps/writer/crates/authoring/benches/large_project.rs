//! Run explicitly with --features benchmarks; emits timings and host memory evidence.
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::{Document, ProjectContext, SearchIndex, workload};
use serde_json::{Value, json};
use std::hint::black_box;

fn measure<T>(name: &str, rows: &mut Vec<Value>, operation: impl FnOnce() -> T) -> T {
    #[cfg(feature = "heap-profile")]
    let before = dhat::HeapStats::get();
    let start = WallTime.start();
    let value = operation();
    let elapsed = WallTime.end(start).as_secs_f64() * 1000.;
    #[cfg(feature = "heap-profile")]
    let after = dhat::HeapStats::get();
    let row = json!({"operation":name, "milliseconds":elapsed});
    #[cfg(feature = "heap-profile")]
    let row = {
        let mut row = row;
        row["allocated_bytes"] = json!(after.total_bytes - before.total_bytes);
        row["allocations"] = json!(after.total_blocks - before.total_blocks);
        row["live_bytes"] = json!(after.curr_bytes);
        row["peak_live_bytes"] = json!(after.max_bytes);
        row
    };
    rows.push(row);
    value
}
#[cfg(feature = "heap-profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let check_heap = args.iter().any(|arg| arg == "--check-heap");
    if check_heap && !cfg!(feature = "heap-profile") {
        return Err("--check-heap requires the heap-profile feature".into());
    }
    #[cfg(feature = "heap-profile")]
    let _profiler = if check_heap {
        dhat::Profiler::builder().testing().build()
    } else {
        dhat::Profiler::new_heap()
    };
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
    if check_heap && (passages != 10_000 || per_document != 500) {
        return Err("heap regression check uses 10,000 passages and 500 per document".into());
    }
    let linked = args.iter().any(|arg| arg == "--linked");
    let mut rows = Vec::new();
    let inputs = measure("generate", &mut rows, || {
        workload::project(passages, per_document).map(|documents| {
            if linked {
                documents
                    .into_iter()
                    .map(|doc| {
                        recite_compiler::authoring::SavedDocument::new(
                            doc.key().clone(),
                            doc.text()
                                .replace("-> END", "-> scene_00000.recite::beat_0"),
                        )
                    })
                    .collect()
            } else {
                documents
            }
        })
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
    for _ in 0..10 {
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
    }
    let peak = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("VmHWM:"))
                .map(str::to_owned)
        });
    #[cfg(feature = "heap-profile")]
    if check_heap {
        // Fixed corpus allocation bounds, independent of wall-clock/host speed.
        // Heap evidence and rationale live in scalability.md.
        let peak_bytes = dhat::HeapStats::get().max_bytes;
        dhat::assert!(peak_bytes < 20_000_000);
        for row in &rows {
            let budget = match row["operation"].as_str() {
                Some("index") => Some(55_000_000),
                Some("project_script") => Some(3_200_000),
                Some("edit" | "undo" | "redo" | "multiline_edit" | "id_edit") => Some(4_000_000),
                _ => None,
            };
            if let Some(budget) = budget {
                dhat::assert!(
                    row["allocated_bytes"]
                        .as_u64()
                        .is_some_and(|bytes| bytes < budget)
                );
            }
        }
    }
    let report = json!({"os":std::env::consts::OS, "arch":std::env::consts::ARCH,
        "profile":"bench", "heap_instrumented":cfg!(feature = "heap-profile"), "shared_destination":linked, "passages":passages, "documents":inputs.len(), "source_bytes":bytes,
        "diagnostic_count":diagnostic_count, "history_bytes":document.history_bytes(), "process_peak_rss":peak, "samples":rows});
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(path) = argument("--output") {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    Ok(())
}
