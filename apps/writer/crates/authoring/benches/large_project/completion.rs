//! Repeated Source queries must preserve the accepted document and saved project.
use super::measure;
use recite_core::SourcePosition;
use recite_writer_model::Document;
use serde_json::Value;

pub(super) fn run(
    document: &Document,
    rows: &mut Vec<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let original = document.source_snapshot();
    let revision = document.revision();
    let jump = original.rfind("-> beat_").ok_or("generated local jump")?;
    let accepted_position = position(&original, jump)?;
    let last_jump = original.rfind("-> ").ok_or("generated final jump")?;
    let draft = format!("{}-> beat_\n", &original[..last_jump]);
    let changed_position = position(&draft, last_jump)?;
    for _ in 0..5 {
        let captured = measure("capture_completion_snapshot", rows, || {
            document.completion_snapshot()
        });
        assert!(captured.is_current(document));
        assert!(captured.same_analysis(&document.completion_snapshot()));
        for (operation, text, cursor) in [
            (
                "accepted_draft_completion",
                original.as_ref(),
                accepted_position,
            ),
            ("changed_draft_completion", draft.as_str(), changed_position),
        ] {
            let result = measure(operation, rows, || document.complete_draft(text, cursor))?;
            assert!(!result.candidates().is_empty());
            for index in [0, result.candidates().len() - 1] {
                let (range, replacement) = result.replacement(text, index)?;
                assert!(text[range].starts_with("beat_"));
                assert!(replacement.starts_with("beat_"));
            }
            assert_eq!(document.revision(), revision);
            assert!(std::sync::Arc::ptr_eq(
                &original,
                &document.source_snapshot()
            ));
        }
    }
    Ok(())
}

fn position(source: &str, jump: usize) -> Result<SourcePosition, Box<dyn std::error::Error>> {
    let line = source[..jump].bytes().filter(|byte| *byte == b'\n').count() + 1;
    Ok(SourcePosition::new(line.try_into()?, 9)?)
}
