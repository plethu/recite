use super::*;

#[test]
fn inconsistent_beat_and_passage_link_does_not_change_selection() -> Result<(), WorkbenchError> {
    let mut model = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let before = model.view().clone();
    let passage = model.document().passage_snapshot()?[0].id.clone();
    let location = Location {
        beat: Some("not_the_passage_beat".into()),
        passage: Some(passage),
        ..Location::default()
    };
    assert!(select(&mut model, &location).is_err());
    assert_eq!(model.view(), &before);
    Ok(())
}
