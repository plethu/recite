//! A single large block must not retain a complete transient AST while indexing.
use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::SearchIndex;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn giant_block_search_bounds_peak_and_preserves_replaced_snapshots()
-> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from(":: giant default\n");
    for n in 0..20_000 {
        source.push_str(&format!("> line_{n}@{n:020x} speaker=mara\n  The courier waits beside the café. Beacon{} remains unanswered.\n\n", n % 4096));
    }
    source.push_str("-> END\n");
    let key = DocumentKey::new("scene.recite")?;
    let original = SavedDocument::new(key.clone(), &source);
    let next = SavedDocument::new(key, source.replacen("courier", "zeppelin", 1));
    let _profiler = dhat::Profiler::builder().testing().build();
    let mut index = SearchIndex::build(&[original]);
    let built = dhat::HeapStats::get();
    dhat::assert!(built.total_bytes < 40_000_000);
    dhat::assert!(built.curr_bytes < 6_000_000);
    dhat::assert!(built.max_bytes < 12_000_000);
    let retained = index.search_results("courier mara", 20_000);
    assert_eq!(retained.total(), 20_000);
    assert_eq!(retained.len(), 20_000);
    for n in [0, 19_999] {
        assert_eq!(
            retained.get(n).ok_or("hit")?.text,
            format!(
                "The courier waits beside the café. Beacon{} remains unanswered.",
                n % 4096
            )
        );
    }
    index.replace_document(next);
    assert_eq!(index.search_results("courier", 0).total(), 19_999);
    assert_eq!(index.search_results("zeppelin", 1).total(), 1);
    assert!(
        retained
            .get(0)
            .ok_or("retained first")?
            .text
            .contains("courier")
    );
    println!(
        "giant block built allocated={} retained={} peak={}",
        built.total_bytes, built.curr_bytes, built.max_bytes
    );
    Ok(())
}
