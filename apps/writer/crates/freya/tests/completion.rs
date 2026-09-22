mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

#[test]
fn completion_replaces_the_diagnostic_token_without_applying_the_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> line@11111111111111111111\n  Hello.\n-> missing\n:: destination\n-> END\n";
    let model = recite_writer_model::Workbench::new(source)?;
    let diagnostics = model.document().diagnostics();
    let diagnostic = diagnostics.first().ok_or("diagnostic")?;
    let span = &diagnostic.span;
    let caption = format!(
        "Open diagnostic {} at {}:{}:{}",
        diagnostic.code,
        span.file,
        span.start.line(),
        span.start.column()
    );
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1200., 900.),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(source.into()));
        },
        1.,
    )
    .0;
    support::click(&mut test, "Source")?;
    support::click(&mut test, &caption)?;
    test.press_key(Key::Named(NamedKey::End));
    test.poll_n(std::time::Duration::from_millis(16), 5);
    let line_area = |test: &TestingRunner| {
        test.find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .map(|s| s.text.as_ref())
                        .collect::<String>()
                        .contains("-> missing")
                })
                .map(|_| node.layout().area)
        })
    };
    let before = line_area(&test).ok_or("source line")?;
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.role() == AccessibilityRole::ListBox))
            .is_none()
    );
    support::click(&mut test, "Complete at cursor")?;
    assert_eq!(line_area(&test), Some(before));
    let picker = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.role() == AccessibilityRole::ListBox)
                .map(|_| node.layout().area)
        })
        .ok_or("completion picker")?;
    assert!((picker.min_y() - before.max_y()).abs() < 5.);
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("source-completion.png"));
    }
    test.press_key(Key::Named(NamedKey::Escape));
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.role() == AccessibilityRole::ListBox))
            .is_none()
    );
    for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
        test.send_event(PlatformEvent::Keyboard {
            name,
            key: Key::Character(" ".into()),
            code: Code::Space,
            modifiers: Modifiers::CONTROL,
        });
        test.poll_n(std::time::Duration::from_millis(16), 6);
    }
    support::click(&mut test, "destination")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains("-> destination")))
            .is_some()
    );
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Apply draft"))
            .is_some()
    );
    Ok(())
}
