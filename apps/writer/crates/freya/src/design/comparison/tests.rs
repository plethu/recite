use super::*;
use freya_testing::prelude::*;

fn comparison() -> impl IntoElement {
    use_init_theme(light_theme);
    ComparisonView {
        code: false,
        before: "Saved version".into(),
        after: "Draft".into(),
        rows: vec![
            ComparisonRow {
                caption: "First change".into(),
                before: Some("Old beginning".into()),
                after: Some("New beginning".into()),
            },
            ComparisonRow {
                caption: "Unchanged context".into(),
                before: Some("Same surrounding line".into()),
                after: Some("Same surrounding line".into()),
            },
            ComparisonRow {
                caption: "Second change".into(),
                before: Some("Removed line".into()),
                after: None,
            },
        ],
    }
}

fn has(test: &TestingRunner, value: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == value))
        .is_some()
}

fn click(test: &mut TestingRunner, value: &str) -> Result<(), String> {
    let area = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some(value))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("Missing {value}"))?;
    test.click_cursor(area.center().to_f64());
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}

#[test]
fn comparison_navigation_keeps_unchanged_context_optional_and_versions_identified()
-> Result<(), String> {
    let mut test = TestingRunner::new(comparison, Size2D::new(1000., 1100.), |_| {}, 1.).0;
    assert!(has(&test, "First change"));
    assert!(has(&test, "Second change"));
    assert!(!has(&test, "Unchanged context"));
    click(&mut test, "Next")?;
    assert!(has(&test, "First change"));
    assert!(!has(&test, "Second change"));
    click(&mut test, "Show unchanged context")?;
    assert!(has(&test, "Unchanged context"));
    click(&mut test, "Next")?;
    assert!(!has(&test, "First change"));
    assert!(has(&test, "Second change"));
    click(&mut test, "Previous")?;
    assert!(has(&test, "First change"));
    click(&mut test, "Show all changes")?;
    assert!(has(&test, "Second change"));
    click(&mut test, "Unified view")?;
    assert!(has(&test, "Saved version"));
    assert!(has(&test, "Draft"));
    click(&mut test, "Side by side")?;
    Ok(())
}
