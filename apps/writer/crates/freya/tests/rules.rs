mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n? reply@22222222222222222222 requires=(standing(3) and has_key(gate))\n  Let me through.\n  -> accepted\n:: accepted\n! immediate play_sfx(gate_latch)\n! blocking mark_map(floodgate)\n-> END\n";

#[test]
fn reply_rules_apply_discard_and_history_use_the_source_draft() -> TestResult {
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        (1400., 1100.).into(),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(SOURCE.into()));
        },
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    open_rules(&mut test)?;
    support::click(&mut test, "Available when: Any is true")?;
    support::click(&mut test, "Apply rules")?;
    support::click(&mut test, "Edit in Source")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains(" or ")))
            .is_some()
    );
    support::click(&mut test, "Back")?;
    support::click(&mut test, "Make always available")?;
    support::click(&mut test, "Discard rule changes")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Available when"))
            .is_some()
    );
    test.render_to_file("/tmp/recite-reply-rules.png");
    support::click(&mut test, "Undo")?;
    support::click(&mut test, "Edit in Source")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains("standing(3) and has_key(gate)")))
            .is_some()
    );
    Ok(())
}

#[test]
fn typed_rule_draft_survives_source_and_history_without_applying() -> TestResult {
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        (1000., 1200.).into(),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(SOURCE.into()));
        },
        1.,
    )
    .0;
    support::dark_theme(&mut test)?;
    support::open_beat(&mut test)?;
    open_rules(&mut test)?;
    let input = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().map(|s| s.text.as_ref()).collect::<String>() == "3")
                .map(|_| node.layout().area)
        })
        .ok_or("numeric rule input")?;
    test.click_cursor((f64::from(input.center().x), f64::from(input.center().y)));
    test.press_key(Key::Named(NamedKey::End));
    test.press_key(Key::Named(NamedKey::Backspace));
    test.write_text("-");
    test.poll_n(std::time::Duration::from_millis(16), 8);
    let disabled = test
        .find(|_, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Apply rules"))
                .map(|r| r.accessibility.builder.is_disabled())
        })
        .ok_or("apply button")?;
    assert!(disabled);
    submit(&mut test);
    test.render_to_file("/tmp/recite-reply-rules-dark.png");
    support::click(&mut test, "Edit in Source")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains("standing(-)")))
            .is_some()
    );
    support::click(&mut test, "Back")?;
    support::click(&mut test, "Discard rule changes")?;
    support::click(&mut test, "Edit in Source")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains("standing(3)")))
            .is_some()
    );
    Ok(())
}

#[test]
fn expanded_effect_edit_and_reorder_keep_the_right_arguments() -> TestResult {
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        (1400., 1500.).into(),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(SOURCE.into()));
        },
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    open_rules(&mut test)?;
    support::click(&mut test, "Condition actions: standing")?;
    support::click(&mut test, "Remove condition standing")?;
    support::click(&mut test, "Edit effect 1")?;
    let input = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans.iter().map(|s| s.text.as_ref()).collect::<String>() == "gate_latch"
                })
                .map(|_| node.layout().area)
        })
        .ok_or("effect input")?;
    test.click_cursor((f64::from(input.center().x), f64::from(input.center().y)));
    test.press_key(Key::Named(NamedKey::End));
    for _ in 0.."gate_latch".len() {
        test.press_key(Key::Named(NamedKey::Backspace));
    }
    test.write_text("bell");
    test.poll_n(std::time::Duration::from_millis(16), 8);
    support::click(&mut test, "Move effect 1 down")?;
    submit(&mut test);
    support::click(&mut test, "Edit in Source")?;
    let source = test
        .find_many(|_, e| {
            Paragraph::try_downcast(e)
                .map(|p| p.spans.iter().map(|s| s.text.as_ref()).collect::<String>())
        })
        .join("\n");
    assert!(!source.contains("standing("));
    assert!(source.contains("has_key(gate)"), "{source}");
    assert!(source.contains("! blocking mark_map(floodgate)\n! immediate play_sfx(bell)"));
    Ok(())
}

fn submit(test: &mut TestingRunner) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.poll_n(std::time::Duration::from_millis(16), 8);
}

fn open_rules(test: &mut TestingRunner) -> TestResult {
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Let me through.")))
                .map(|_| node.layout().area)
        })
        .ok_or("reply text")?;
    test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 6);
    support::click(test, "Passage actions")?;
    support::click(test, "Reply rules")
}
