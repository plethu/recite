use super::*;

pub(super) fn set_fonts(test: &mut TestingRunner) {
    #[expect(
        clippy::disallowed_types,
        reason = "Freya TestingRunner::set_fonts requires HashMap."
    )]
    let fonts = std::collections::HashMap::from([
        (
            "Fraunces 72pt Soft",
            &include_bytes!("../../fonts/fraunces/Fraunces72ptSoft-SemiBold.ttf")[..],
        ),
        (
            "IBM Plex Sans",
            &include_bytes!("../../fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf")[..],
        ),
        (
            "IBM Plex Mono",
            &include_bytes!("../../fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf")[..],
        ),
        (
            "Literata",
            &include_bytes!("../../fonts/literata/Literata-Variable.ttf")[..],
        ),
    ]);
    test.set_fonts(fonts);
    test.sync_and_update();
}

#[test]
fn project_toolbar_controls_do_not_overlap_with_bundled_fonts()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    for width in [1280., 1440.] {
        let mut test = TestingRunner::new(
            recite_writer::editor_app,
            Size2D::new(width, 720.),
            |_| {},
            1.,
        )
        .0;
        set_fonts(&mut test);
        fill(&mut test, "Project folder", &dir.path().to_string_lossy())?;
        support::click(&mut test, "Open project")?;
        support::wait_editor_work(&mut test)?;
        for _ in 0..6 {
            test.poll_n(std::time::Duration::from_millis(16), 2);
        }
        let areas = std::cell::RefCell::new(Vec::new());
        test.find(|node, element| {
            if let Some(rect) = Rect::try_downcast(element)
                && let Some(label) = rect.accessibility.builder.label()
                && (label == "Commands"
                    || label == "Save"
                    || ["Script", "Map", "Source"]
                        .iter()
                        .any(|view| label.ends_with(&format!(": {view}"))))
            {
                areas
                    .borrow_mut()
                    .push((label.to_owned(), node.layout().area));
            }
            None::<()>
        });
        let areas = areas.into_inner();
        assert!(areas.len() >= 4, "toolbar controls missing: {areas:?}");
        for (index, (name, area)) in areas.iter().enumerate() {
            assert!(
                area.min_x() >= 0. && area.max_x() <= width,
                "{width}: {name} outside window: {area:?}"
            );
            for (other, other_area) in &areas[index + 1..] {
                let overlaps = area.min_x() < other_area.max_x()
                    && area.max_x() > other_area.min_x()
                    && area.min_y() < other_area.max_y()
                    && area.max_y() > other_area.min_y();
                assert!(
                    !overlaps,
                    "{width}: {name} {area:?} overlaps {other} {other_area:?}"
                );
            }
        }
        if let Ok(directory) = std::env::var("RECITE_TOOLBAR_SCREENSHOTS") {
            test.render_to_file(format!("{directory}/toolbar-{width}.png"));
        }
    }
    Ok(())
}
