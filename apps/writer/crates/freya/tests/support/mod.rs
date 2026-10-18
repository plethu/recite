#![allow(dead_code)]
use freya::prelude::*;
use freya_testing::prelude::*;
pub fn wait_for(
    test: &mut TestingRunner,
    description: &str,
    ready: impl Fn(&TestingRunner) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..10_000 {
        test.sync_and_update();
        if ready(test) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Err(format!("Timed out waiting for {description}").into())
}
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
    if let Ok(directory) = std::env::var("RECITE_WRITER_PENDING_SCREENSHOTS") {
        test.sync_and_update();
        let pending = test
            .find(|_, e| {
                Label::try_downcast(e).filter(|l| {
                    l.text.starts_with("Preparing navigation…")
                        || l.text.starts_with("Preparing passage edit…")
                        || l.text.starts_with("Preparing reply rules…")
                        || l.text.starts_with("Preparing source edit…")
                        || l.text.starts_with("Updating document history…")
                        || l.text.starts_with("Saving translations…")
                        || l.text.starts_with("Updating reply rules…")
                        || l.text.starts_with("Saving reply rules…")
                })
            })
            .is_some();
        if pending {
            let name = caption.replace(' ', "-").to_lowercase();
            test.render_to_file(format!("{directory}/{name}.png"));
        }
    }
    test.poll_n(std::time::Duration::from_millis(16), 15);
    if matches!(caption, "Try scene" | "Try this scene" | "Restart preview") {
        wait_preview(test)?;
    }
    wait_editor_work(test)?;
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

/// Pump the component result task until startup settles; simulated frames alone
/// do not wait for an OS worker on a busy machine.
pub fn wait_preview(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..10_000 {
        test.sync_and_update();
        if test
            .find(|_, e| {
                Label::try_downcast(e)
                    .filter(|l| l.text.as_ref() == "Preparing preview… You can keep editing.")
            })
            .is_none()
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Err("preview preparation did not settle".into())
}

/// Editor completion follows worker receipts, not a number of simulated frames.
pub fn wait_editor_work(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..10_000 {
        test.sync_and_update();
        let pending = test
            .find(|_, e| {
                Label::try_downcast(e).filter(|l| {
                    [
                        "Saving… You can keep editing.",
                        "Saving translations… You can keep editing.",
                        "Preparing source edit… You can keep editing.",
                        "Preparing passage edit… You can keep editing.",
                        "Preparing navigation… You can keep editing.",
                        "Updating document history…",
                        "Preparing reply rules…",
                        "Updating reply rules…",
                        "Saving reply rules…",
                    ]
                    .iter()
                    .any(|prefix| l.text.starts_with(prefix))
                })
            })
            .is_some();
        if !pending {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Err("editor work did not settle".into())
}
