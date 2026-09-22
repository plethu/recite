use recite_core::SourcePosition;
use recite_writer_model::Document;

#[test]
fn completion_queries_the_draft_and_refuses_a_stale_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(":: start default\n-> END\n")?;
    let draft = ":: start default\n-> dest\n:: destination\n-> END\n";
    let completions = document.complete_draft(draft, SourcePosition::new(2, 8)?)?;
    let index = completions
        .candidates()
        .iter()
        .position(|c| c.name() == "destination")
        .ok_or("destination")?;
    let (range, replacement) = completions.replacement(draft, index)?;
    let mut completed = draft.to_owned();
    completed.replace_range(range, replacement);
    assert!(completed.contains("-> destination\n"), "{completed:?}");
    assert!(completions.replacement(&completed, index).is_err());
    assert_eq!(document.source(), ":: start default\n-> END\n");
    Ok(())
}
