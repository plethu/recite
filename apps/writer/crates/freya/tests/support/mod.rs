#![allow(dead_code)]
use freya::prelude::*;
use freya_testing::prelude::*;
pub fn click(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some(caption))
                .map(|_| node.layout().area)
        })
        .or_else(|| {
            test.find(|node, element| {
                Label::try_downcast(element)
                    .filter(|l| l.text.as_ref() == caption)
                    .map(|_| node.layout().area)
            })
        })
        .ok_or_else(|| format!("Missing {caption}"))?;
    test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 15);
    Ok(())
}
pub fn open_beat(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    click(test, "Map")?;
    let area = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| {
                    r.accessibility
                        .builder
                        .label()
                        .is_some_and(|l| l.starts_with("Select "))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("beat card")?;
    // A graph card can extend below the viewport in a small window. Its title
    // remains visible; the geometric centre may be behind the connections pane.
    test.click_cursor((f64::from(area.center().x), f64::from(area.min_y() + 20.)));
    test.poll_n(std::time::Duration::from_millis(16), 8);
    click(test, "Open script")?;
    Ok(())
}
pub fn dark_theme(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    click(test, "Settings")?;
    click(test, "Theme: Dark")?;
    click(test, "Done")
}
