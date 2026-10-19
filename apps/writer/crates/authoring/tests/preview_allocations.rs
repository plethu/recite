//! UI submission must share accepted analysis rather than clone project inputs.
use recite_writer_model::{Document, View, Workbench};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn cold_preview_capture_cost_is_independent_of_source_size()
-> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from(":: giant default\n");
    for n in 0..20_000 {
        source.push_str(&format!(
            "> line_{n}@{n:020x}\n  The courier waits beside the café.\n"
        ));
    }
    source.push_str("-> END\n");
    let model = Workbench::from_document(Document::new(source)?, Some(View::Source))?;
    let mut requests = Vec::with_capacity(100);
    let _profiler = dhat::Profiler::builder().testing().build();
    for _ in 0..100 {
        requests.push(model.preview_request()?);
    }
    let captured = dhat::HeapStats::get();
    // Only the small document key is copied. No source, document vector, AST,
    // projection, compilation or history allocation belongs to capture.
    dhat::assert!(captured.total_bytes < 4_096);
    dhat::assert!(captured.total_blocks <= 100);
    assert!(requests.iter().all(|request| request.is_current(&model)));
    Ok(())
}
