use crate::project::sessions::tests::two_scenes;
use recite_writer_model::View;

#[test]
fn edit_then_revert_cannot_reuse_an_older_parking_receipt() -> Result<(), Box<dyn std::error::Error>>
{
    let (_dir, mut files, mut model, other) = two_scenes()?;
    model.select(View::Source)?;
    let mut prepared = files.switch_request(&model, &other, None)?.prepare(
        &recite_compiler::authoring::CancellationToken::new(),
        |_| Ok(()),
    )?;
    let checkpoint = files.begin_navigation_checkpoint(&model)?.wait()?;
    let draft = model.draft().to_owned();
    model.set_draft("New draft after parking".into());
    files.queue_checkpoint(&model)?;
    model.set_draft(draft);
    files.queue_checkpoint(&model)?;
    // Identical current bytes do not establish that the replacement worker exited.
    assert!(
        files
            .accept_switch(&mut model, &mut prepared, &checkpoint)
            .is_err()
    );
    let checkpoint = files.begin_navigation_checkpoint(&model)?.wait()?;
    files.accept_switch(&mut model, &mut prepared, &checkpoint)?;
    assert_eq!(files.current, other);
    Ok(())
}
