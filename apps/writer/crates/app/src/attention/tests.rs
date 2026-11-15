use crate::test_support::{open, project};

#[test]
fn the_scene_list_hides_by_choice_narrows_below_1100_and_yields_to_focus_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n")?;
    let mut writer = open(&directory)?;
    writer.viewport.width = 1200.;
    assert_eq!(writer.navigation_width(), writer.drawer_width());
    writer.viewport.width = 900.;
    assert_eq!(
        writer.navigation_width(),
        writer.drawer_width().min(crate::NARROW_SIDEBAR)
    );
    writer.toggle_sidebar();
    assert!(writer.attention.sidebar_hidden());
    assert_eq!(writer.navigation_width(), 0.);
    writer.toggle_sidebar();
    assert_eq!(writer.navigation_width(), crate::NARROW_SIDEBAR);
    writer.toggle_focus();
    assert_eq!(writer.navigation_width(), 0.);
    // Focus mode owns the chrome; the choice to hide the list waits for it to end.
    writer.toggle_sidebar();
    assert!(!writer.attention.sidebar_hidden());
    Ok(())
}
