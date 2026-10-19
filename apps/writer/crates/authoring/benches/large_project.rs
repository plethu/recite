//! Run explicitly with --features benchmarks; emits timings and host memory evidence.
#[path = "large_project/completion.rs"]
mod completion;
#[path = "large_project/drafts.rs"]
mod drafts;
#[path = "large_project/preview.rs"]
mod preview;
#[path = "large_project/rules.rs"]
mod rules;
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::{Document, ProjectContext, SearchIndex, View, Workbench, workload};
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
    let index = measure("index", &mut rows, || {
        SearchIndex::build_with_control(
            &inputs,
            &recite_compiler::authoring::CancellationToken::new(),
        )
    })?;
    assert_eq!(index.len(), passages);
    for _ in 0..5 {
        for (name, query, limit) in [
            ("common_search", "courier", 100),
            ("common_multiword_search", "courier mara", 100),
            ("expanded_search", "courier", 10_000),
        ] {
            let hits = measure(name, &mut rows, || {
                index.search_results(black_box(query), limit)
            });
            assert_eq!(hits.total(), passages);
            assert_eq!(hits.len(), passages.min(limit));
            measure("clone_search_results", &mut rows, || {
                black_box(hits.clone())
            });
        }
    }
    for _ in 0..5 {
        let (count, _) = measure("search", &mut rows, || {
            index.search(black_box("mara beacon17"), 50)
        });
        assert!(count > 0 || passages <= 17);
        measure("clone_saved_inputs", &mut rows, || {
            black_box(inputs.clone())
        });
    }
    let mut replaced = index.clone();
    let replacement = recite_compiler::authoring::SavedDocument::new(
        inputs[0].key().clone(),
        inputs[0].text().replacen("courier", "zeppelin", 1),
    );
    measure("replace_search_region", &mut rows, || {
        replaced.replace_document(replacement)
    });
    assert_eq!(replaced.search_results("zeppelin", 1).total(), 1);
    drop(replaced);
    let preview_source = workload::source(500, 0, 5);
    let preview_document = Document::new(&preview_source)?;
    for _ in 0..3 {
        let mut preview = measure("preview_start", &mut rows, || {
            recite_writer_model::Preview::new(&preview_document).map_err(Box::new)
        })?;
        assert!(preview.advance(None)?.text.contains("courier"));
    }
    drop(preview_document);
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
    completion::run(&document, &mut rows)?;
    for _ in 0..5 {
        let (count, endings) = measure("source_navigation_facts", &mut rows, || {
            let blocks = document.block_summaries();
            (
                blocks.len(),
                blocks
                    .iter()
                    .filter(|block| block.has_end_destination())
                    .count(),
            )
        });
        assert_eq!(count, per_document.min(passages).div_ceil(5));
        assert_eq!(endings, usize::from(!linked));
    }
    for _ in 0..5 {
        let summary = measure("capture_navigation_summary", &mut rows, || {
            document.summary_snapshot()
        })
        .ok_or("missing accepted summary")?;
        assert_eq!(summary.blocks(), document.block_summaries());
        assert!(std::sync::Arc::ptr_eq(
            &summary,
            &document.summary_snapshot().ok_or("missing summary")?
        ));
    }
    for _ in 0..5 {
        let snapshot = measure("capture_diagnostics", &mut rows, || {
            document.diagnostic_snapshot()
        });
        let cloned = measure("clone_diagnostics", &mut rows, || snapshot.clone());
        assert!(snapshot == cloned);
        assert_eq!(snapshot.len(), diagnostic_count);
        assert!(snapshot.get(diagnostic_count).is_none());
    }
    let bindings = measure("preview_bindings", &mut rows, || {
        document.preview_bindings()
    });
    assert!(bindings.is_empty()); // This fixed prose corpus has no caller variables.
    for _ in 0..5 {
        let cached = measure("capture_preview_bindings", &mut rows, || {
            document.preview_bindings()
        });
        assert!(std::sync::Arc::ptr_eq(&bindings, &cached));
    }
    let first = measure("project_script", &mut rows, || document.script_snapshot())?;
    for _ in 0..5 {
        let cached = measure("cached_script", &mut rows, || document.script_snapshot())?;
        assert!(std::sync::Arc::ptr_eq(&first, &cached));
    }
    let original_source = document.source_snapshot();
    let first_passage = document.passage_snapshot()?[0].clone();
    for edit in 0..10 {
        let text = format!("{} Edited {edit} 🐧.", first_passage.text);
        let (script, selected) = measure("prose_edit_and_script", &mut rows, || {
            document.replace_text(document.revision(), &first_passage.id, &text)?;
            Ok::<_, recite_writer_model::EditError>((
                document.script_snapshot()?,
                document.find_passage(&first_passage.id)?,
            ))
        })?;
        assert_eq!(selected.ok_or("missing passage")?.text, text);
        black_box(script);
    }
    for _ in 0..10 {
        assert!(document.undo()?);
    }
    assert_eq!(document.source(), original_source.as_ref());
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
    let history_bytes = document.history_bytes();
    let mut workbench = measure("open_source_workbench", &mut rows, || {
        Workbench::from_document(document, Some(View::Source))
    })?;
    workbench.select(View::Source)?;
    workbench.set_draft(format!("{}\nRejected draft", workbench.draft()));
    for _ in 0..100 {
        let snapshot = measure("draft_snapshot", &mut rows, || workbench.recovery());
        let queued = measure("clone_draft_snapshot", &mut rows, || snapshot.clone());
        assert_eq!(queued, snapshot);
    }
    drafts::run(&mut workbench, &mut rows)?;
    rules::run(&mut rows)?;
    preview::catalogue(&mut rows)?;
    preview::bindings(&mut rows)?;
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
        // Heap evidence and rationale live in the profiling guide and Writer acceptance.
        let peak_bytes = dhat::HeapStats::get().max_bytes;
        dhat::assert!(peak_bytes < 20_000_000);
        for row in &rows {
            let budget = match row["operation"].as_str() {
                Some("index") => {
                    // Lower one grammar-safe region at a time, rather than
                    // retaining a whole document's transient lowered AST.
                    dhat::assert!(
                        row["peak_live_bytes"]
                            .as_u64()
                            .is_some_and(|bytes| bytes < 6_000_000)
                    );
                    dhat::assert!(
                        row["live_bytes"]
                            .as_u64()
                            .is_some_and(|bytes| bytes < 5_300_000),
                        "retained index bytes={}",
                        row["live_bytes"]
                    );
                    Some(26_000_000)
                }
                Some("replace_search_region") => Some(350_000),
                Some("preview_start") => Some(3_900_000),
                Some("preview_bindings") => Some(128),
                Some("first_preview_bindings") => Some(512),
                Some("preview_catalogue_start") => {
                    dhat::assert!(
                        row["allocations"]
                            .as_u64()
                            .is_some_and(|count| count <= 250)
                    );
                    Some(25_000)
                }
                Some("project_script") => Some(3_200_000),
                Some("prose_edit_and_script") => Some(1_200_000),
                Some("accepted_draft_completion") => Some(35_000),
                Some("changed_draft_completion") => Some(1_600_000),
                Some("source_rope_apply") => {
                    dhat::assert!(
                        row["allocations"]
                            .as_u64()
                            .is_some_and(|count| count <= 350)
                    );
                    Some(325_000)
                }
                Some("rules_clean_changed" | "rules_condition_changed") => {
                    dhat::assert!(row["allocations"].as_u64() == Some(0));
                    Some(1)
                }
                Some("rules_snapshot") => Some(2_000),
                Some("source_edit_capture") => {
                    dhat::assert!(row["allocations"].as_u64().is_some_and(|count| count <= 1));
                    Some(128)
                }
                Some("source_edit_accept") => Some(1024),
                Some("passage_history_capture") => {
                    dhat::assert!(row["allocations"].as_u64().is_some_and(|count| count <= 3));
                    Some(128)
                }
                Some("passage_history_accept") => {
                    dhat::assert!(row["allocations"].as_u64().is_some_and(|count| count <= 1));
                    Some(128)
                }
                Some("passage_history_prepare") => Some(3_300_000),
                Some("source_edit_prepare") => Some(325_000),
                Some("capture_completion_snapshot") => {
                    dhat::assert!(row["allocations"].as_u64().is_some_and(|count| count <= 1));
                    Some(128)
                }
                Some("edit" | "undo" | "redo" | "multiline_edit" | "id_edit") => Some(4_000_000),
                Some("common_search" | "common_multiword_search") => Some(20_000),
                Some("expanded_search") => Some(750_000),
                Some("source_widget_snapshot" | "source_widget_equal_length_snapshot") => {
                    dhat::assert!(row["allocations"].as_u64().is_some_and(|count| count <= 2));
                    Some(256)
                }
                Some(
                    "clone_search_results"
                    | "capture_diagnostics"
                    | "clone_diagnostics"
                    | "draft_snapshot"
                    | "clone_draft_snapshot"
                    | "select_source_snapshot"
                    | "open_source_workbench"
                    | "source_navigation_facts"
                    | "capture_navigation_summary"
                    | "capture_preview_bindings",
                ) => {
                    dhat::assert!(row["allocations"].as_u64() == Some(0));
                    Some(1)
                }
                _ => drafts::budget(row),
            };
            if let Some(budget) = budget {
                dhat::assert!(
                    row["allocated_bytes"]
                        .as_u64()
                        .is_some_and(|bytes| bytes < budget),
                    "operation={} allocated_bytes={} budget={budget}",
                    row["operation"],
                    row["allocated_bytes"]
                );
            }
        }
    }
    let report = json!({"os":std::env::consts::OS, "arch":std::env::consts::ARCH,
        "profile":"bench", "search_index_builder":"cooperative", "heap_instrumented":cfg!(feature = "heap-profile"), "shared_destination":linked, "passages":passages, "documents":inputs.len(), "source_bytes":bytes,
        "diagnostic_count":diagnostic_count, "history_bytes":history_bytes, "process_peak_rss":peak, "samples":rows});
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(path) = argument("--output") {
        std::fs::write(path, &text)?;
    }
    println!("{text}");
    Ok(())
}
