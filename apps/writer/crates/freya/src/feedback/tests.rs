use super::*;
use freya_testing::prelude::*;
fn form() -> impl IntoElement {
    use_init_theme(light_theme);
    let mut feedback = Feedback::new();
    let mut retried = use_state(|| false);
    use_hook(move || {
        feedback.error_with_action(
            "Save failed".into(),
            "Retry".into(),
            EventHandler::new(move |()| {
                retried.set(true);
                feedback.report(Ok(()), "Saved".into());
            }),
        )
    });
    rect()
        .width(Size::fill())
        .child(NoticeView { feedback })
        .child(label().text(format!("Retried: {}", retried.read())))
}
fn click(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    test.poll_n(std::time::Duration::from_millis(16), 4);
    let area = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some(caption))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("button {caption}"))?;
    test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}
#[test]
fn notice_retry_and_dismiss_have_distinct_effects() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(form, Size2D::new(700., 300.), |_| {}, 1.).0;
    click(&mut test, "Dismiss message")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Retried: false"))
            .is_some()
    );
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Save failed"))
            .is_none()
    );
    let mut test = TestingRunner::new(form, Size2D::new(700., 300.), |_| {}, 1.).0;
    click(&mut test, "Retry")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Retried: true"))
            .is_some()
    );
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.role() == AccessibilityRole::Status))
            .is_some()
    );
    Ok(())
}
