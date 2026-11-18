use crate::test_support as support;
use crate::workspaces::Screen;

#[test]
fn save_is_primary_only_for_unsaved_work_in_a_writing_view()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = support::project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n")?;
    let mut writer = support::open(&directory)?;
    assert!(!writer.save_is_primary(), "clean work keeps Save tonal");
    writer.change(|model| {
        model.set_draft("Unapplied prose.".into());
        Ok(())
    });
    assert!(writer.save_is_primary());
    writer.trial.visible = true;
    assert!(!writer.save_is_primary(), "Preview keeps its own primary");
    writer.trial.visible = false;
    writer.workspaces.screen = Screen::Localisation;
    assert!(!writer.save_is_primary(), "Localise keeps its own primary");
    Ok(())
}
