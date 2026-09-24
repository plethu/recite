//! Deterministic, generated authoring workloads; compiled only for benchmarks.
use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;

pub fn source(passages: usize, first_id: usize, per_beat: usize) -> String {
    let mut source = String::new();
    for passage in 0..passages {
        let beat = first_id + passage / per_beat;
        if passage.is_multiple_of(per_beat) {
            if passage > 0 {
                source.push_str(&format!("-> beat_{beat}\n\n"));
            }
            source.push_str(&format!(
                ":: beat_{beat}{}\n",
                if passage == 0 && first_id == 0 {
                    " default"
                } else {
                    ""
                }
            ));
        }
        let id = first_id + passage;
        source.push_str(&format!("> line_{id}@{id:020x} speaker=mara\n  The courier waits beside the café. Beacon{} remains unanswered.\n\n", id % 4096));
    }
    source.push_str("-> END\n");
    source
}
pub fn project(
    passages: usize,
    per_document: usize,
) -> Result<Vec<SavedDocument>, recite_core::DocumentKeyError> {
    (0..passages.div_ceil(per_document))
        .map(|index| {
            let start = index * per_document;
            Ok(SavedDocument::new(
                DocumentKey::new(format!("scene_{index:05}.recite"))?,
                source((passages - start).min(per_document), start, 5),
            ))
        })
        .collect()
}
