use super::*;
use recite_writer_model::View;

#[test]
fn source_hint_only_initializes_new_sessions_and_does_not_inherit_the_previous_view()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    let first = current.view().clone();
    files.switch(&mut current, &other, Some(View::Source), |_| Ok(()))?;
    assert_eq!(current.view(), &View::Source);
    assert_eq!(
        current.draft().as_ptr(),
        current.document().source().as_ptr()
    );
    current.inspect_block("other")?;
    files.switch(&mut current, &original, Some(View::Source), |_| Ok(()))?;
    assert_eq!(
        current.view(),
        &first,
        "retained selection overrides the hint"
    );
    files.switch(&mut current, &other, Some(View::Source), |_| Ok(()))?;
    assert_eq!(current.view(), &View::Block("other".into()));
    current.select(View::Source)?;
    let third = dir.path().join("c.recite");
    std::fs::write(&third, ":: third\n-> END\n")?;
    files.refresh(&mut current)?;
    files.switch(&mut current, &third, None, |_| Ok(()))?;
    assert_eq!(current.view(), &View::Block("third".into()));
    Ok(())
}

#[test]
fn source_hint_preserves_recovered_rejected_prose_and_explicit_passage_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files, mut current, other) = two_scenes()?;
    let original = files.current.clone();
    let source = ":: other\n> line@11111111111111111111\n  Before.\n-> END\n";
    std::fs::write(&other, source)?;
    let selected = View::Passage("11111111111111111111".into());
    files.switch(&mut current, &other, None, |m| m.select(selected.clone()))?;
    assert_eq!(current.view(), &selected);
    let rejected = "> intruder\n  Preserve this rejected prose.";
    current.set_draft(rejected.into());
    assert!(current.apply().is_err());
    files.switch(&mut current, &original, None, |_| Ok(()))?;
    drop(files);
    let mut files = ProjectFiles::open(dir.path())?;
    let mut current = files.workbench()?;
    files.switch(&mut current, &other, Some(View::Source), |_| Ok(()))?;
    assert_eq!(current.view(), &selected);
    assert_eq!(current.draft(), rejected);
    assert_eq!(current.document().source(), source);
    Ok(())
}
