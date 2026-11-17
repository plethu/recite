use super::*;

#[test]
fn the_spacing_scale_is_the_agreed_seven_steps() {
    use space::*;
    let scale = [UNIT, SIBLINGS, TEXT, KINDS, GROUPS, REGIONS, BEATS];
    assert_eq!(scale, [4., 8., 12., 16., 24., 32., 48.]);
}

#[test]
fn interface_roles_scale_with_the_interface_and_keep_their_leading() {
    for (role, size, line) in [
        (Type::Display, 28., 32.),
        (Type::Section, 20., 24.),
        (Type::Control, 14., 20.),
        (Type::Body, 14., 20.),
        (Type::Meta, 12., 16.),
        (Type::Id, 12., 16.),
    ] {
        assert_eq!(role.metrics(), (size, line), "{role:?}");
        assert_eq!(role.size(2.), 2. * size, "{role:?}");
        assert!(line > size, "{role:?} keeps leading");
    }
    // Hierarchy is weight and size, never a light heading.
    for role in [Type::Display, Type::Section] {
        assert_eq!(role.weight(), FontWeight::SEMI_BOLD);
        assert_eq!(role.families(), FONT_DISPLAY);
    }
    assert_eq!(Type::Id.families(), FONT_MONO);
    assert_eq!(Type::Meta.families(), FONT_UI);
}

#[test]
fn sheet_roles_follow_the_reading_size_but_never_fall_below_their_floors() {
    let page = |reading| Page::new(reading, 1.);
    assert_eq!(Sheet::Dialogue.size(page(20.)), 20.);
    assert_eq!(Sheet::Label.size(page(20.)), 12.);
    assert!((Sheet::Destination.size(page(20.)) - 14.4).abs() < 1e-4);
    // At the smallest reading sizes labels and destinations keep 11 and 13 pixels.
    for reading in [10., 12., 14., 16.] {
        assert!(Sheet::Label.size(page(reading)) >= 11.);
        assert!(Sheet::Destination.size(page(reading)) >= 13.);
    }
    assert!((Sheet::Reply.size(page(20.)) - 18.).abs() < 1e-4);
    assert_eq!(Sheet::text(Some(1)), Sheet::Reply);
    assert_eq!(Sheet::text(None), Sheet::Dialogue);
    assert!((Sheet::Label.tracking(page(20.)) - 0.72).abs() < 1e-4);
    assert_eq!(Sheet::Destination.tracking(page(20.)), 0.);
}

#[test]
fn the_ui_scale_zooms_every_sheet_role_and_its_floors() {
    let roles = [
        Sheet::Dialogue,
        Sheet::Reply,
        Sheet::Number,
        Sheet::Label,
        Sheet::Destination,
    ];
    for reading in [12., 14., 20., 28.] {
        let normal = Page::new(reading, 1.);
        let zoomed = Page::new(reading, 2.);
        for role in roles {
            assert!((role.size(zoomed) - 2. * role.size(normal)).abs() < 1e-4);
        }
        assert!((zoomed.scale() - 2. * normal.scale()).abs() < 1e-4);
        assert!((zoomed.folio() - 2. * normal.folio()).abs() < 1e-4);
        // Controls on the sheet zoom with it and never fall below the interface's size.
        assert!(zoomed.control() >= 2. && normal.control() >= 1.);
        assert!(zoomed.target() >= 48. && normal.target() >= size::TARGET);
    }
}

#[test]
fn the_scene_title_beat_headings_and_folio_follow_the_reading_size_zoomed() {
    for zoom in [1., 1.5, 2.] {
        for reading in [14., 20., 28.] {
            let page = Page::new(reading, zoom);
            assert!((Type::Display.size(page.title()) - 1.6 * reading * zoom).abs() < 1e-4);
            // Beat headings sit one step above the lines they head.
            assert!((Type::Section.size(page.section()) - 1.2 * reading * zoom).abs() < 1e-4);
            assert!(Type::Section.size(page.section()) > Sheet::Dialogue.size(page));
            assert!(Type::Display.size(page.title()) > Type::Section.size(page.section()));
            // The folio is 0.6R, held at 11px at small reading sizes, zoomed.
            let folio = Type::Id.size(page.folio());
            assert!((folio - (0.6 * reading).max(11.) * zoom).abs() < 1e-4);
        }
    }
}

#[test]
fn the_manuscript_line_box_is_taller_than_the_reference_at_every_reading_size() {
    // The comparison reference set lines and replies at R with a 1.2 line box.
    let leading = 1.45;
    assert_eq!(Sheet::LEADING, LineHeight::FontSizeRelative(leading));
    for reading in [12., 14., 16., 20., 24., 32.] {
        for role in [Sheet::Dialogue, Sheet::Reply] {
            assert!(
                role.size(Page::new(reading, 1.)) * leading >= reading * 1.2,
                "{role:?} at {reading}"
            );
        }
    }
    // Fields set in Body keep a line at least as tall as the 16px text at 1.2 they replaced.
    let (_, line) = Type::Body.metrics();
    assert!(line >= 16. * 1.2);
}
