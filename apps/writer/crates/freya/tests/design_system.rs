//! Native component evidence and shared keyboard behaviour, without project I/O.
mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn capture(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
    Ok(())
}

#[test]
fn specimen_uses_live_controls_and_bounded_picklists() -> Result<(), Box<dyn std::error::Error>> {
    for (width, height) in [(1200., 900.), (900., 650.)] {
        let mut test = TestingRunner::new(
            recite_writer::design_app,
            Size2D::new(width, height),
            |_| {},
            1.,
        )
        .0;
        test.poll_n(std::time::Duration::from_millis(16), 12);
        capture(&mut test, &format!("components-dark-{width}.png"))?;
        if std::env::var_os("RECITE_WRITER_CAPTURE_DIR").is_some() && width == 1200. {
            let source = test
                .find(|node, element| {
                    Rect::try_downcast(element)
                        .filter(|rect| {
                            rect.accessibility.builder.label() == Some("Writing view: Source")
                        })
                        .map(|_| node.layout().area)
                })
                .ok_or("Source segment")?;
            test.click_cursor(source.center().to_f64());
            for frame in 0..18 {
                capture(&mut test, &format!("selection-{frame:02}.png"))?;
                test.poll_n(std::time::Duration::from_millis(16), 1);
            }
            support::click(&mut test, "Writing view: Map")?;
        }
        support::click(&mut test, "Try scene")?;
        assert!(
            test.find(
                |_, e| Label::try_downcast(e).filter(|l| l.text == "Primary action activated.")
            )
            .is_some()
        );
        let dark_font = prose_font(&test).ok_or("dark prose font")?;
        support::click(&mut test, "Light appearance")?;
        let light_font = prose_font(&test).ok_or("light prose font")?;
        assert_eq!(
            dark_font, light_font,
            "appearance must not change the resolved prose face"
        );
        println!("Resolved prose font: {light_font:?}");
        capture(&mut test, &format!("components-light-{width}.png"))?;
        support::click(&mut test, "Change destination")?;
        assert!(
            test.find(|_, element| Label::try_downcast(element)
                .filter(|label| label.text == "Choose a beat"))
                .is_none()
        );
        test.write_text("Station");
        test.poll_n(std::time::Duration::from_millis(16), 6);
        let popup = test
            .find(|node, element| {
                Rect::try_downcast(element)
                    .filter(|r| r.accessibility.builder.role() == AccessibilityRole::ListBox)
                    .map(|_| node.layout().area)
            })
            .ok_or("destination list")?;
        assert!(popup.min_x() >= 0. && popup.max_x() <= width);
        assert!(popup.min_y() >= 0. && popup.max_y() <= height);
        assert!(popup.width() <= 400.);
        capture(&mut test, &format!("components-picker-{width}.png"))?;
        test.send_event(PlatformEvent::Keyboard {
            name: KeyboardEventName::KeyDown,
            key: Key::Named(NamedKey::Enter),
            code: Code::Enter,
            modifiers: Modifiers::empty(),
        });
        test.poll_n(std::time::Duration::from_millis(16), 6);
        assert!(
            test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == "Station History"))
                .is_some()
        );
        assert!(
            test.find(|_, e| Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.role() == AccessibilityRole::ListBox))
                .is_none()
        );
    }
    Ok(())
}

fn prose_font(test: &TestingRunner) -> Option<(String, String, f32, bool)> {
    test.find(|node, element| {
        Label::try_downcast(element)
            .filter(|label| label.text.starts_with("If you're here about"))?;
        let layout = node.layout();
        let paragraph = layout
            .data
            .as_ref()?
            .downcast_ref::<skia_safe::textlayout::Paragraph>()?;
        let font = paragraph.get_font_at(0);
        Some((
            font.typeface().family_name(),
            format!("{:?}", font.typeface().font_style()),
            font.size(),
            font.is_embolden(),
        ))
    })
}
