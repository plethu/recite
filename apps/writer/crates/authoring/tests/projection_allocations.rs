use recite_writer_model::Document;
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn giant_projection_capture_and_publication_do_no_source_sized_ui_work()
-> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from(":: giant default\n");
    for n in 0..20_000 {
        source.push_str(&format!(
            "> line_{n}@{n:020x}\n  The courier waits beside the café.\n"
        ));
    }
    source.push_str("-> END\n");
    let document = Document::new(source)?;
    let mut requests = Vec::with_capacity(100);
    let _profiler = dhat::Profiler::builder().testing().build();
    for _ in 0..100 {
        assert!(document.cached_passages().is_none());
        assert!(document.cached_script().is_none());
        requests.push(document.projection_request());
    }
    let captured = dhat::HeapStats::get();
    // Match the existing immutable-preview capture bound: only the key is copied.
    dhat::assert!(captured.total_bytes < 4_096);
    dhat::assert!(captured.total_blocks <= 100);
    assert!(requests.iter().all(|r| r.is_current(&document)));
    let prepared = requests[0].prepare()?;
    let before = dhat::HeapStats::get();
    assert!(document.accept_projection(&prepared));
    let passages = document.cached_passages().ok_or("passages")?;
    let script = document.cached_script().ok_or("script")?;
    for _ in 0..100 {
        assert!(document.accept_projection(&prepared));
        assert!(document.find_passage("00000000000000004e1f")?.is_some());
    }
    let after = dhat::HeapStats::get();
    dhat::assert_eq!(after.total_bytes, before.total_bytes);
    dhat::assert_eq!(after.total_blocks, before.total_blocks);
    assert_eq!(passages.len(), 20_000);
    assert_eq!(script.len(), 1);
    assert_eq!(passages.last().ok_or("last")?.label, "line_19999");
    Ok(())
}
