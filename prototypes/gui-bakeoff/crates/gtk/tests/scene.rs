use gtk::prelude::*;

fn descendants(widget: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut result = vec![widget.clone()];
    let mut child = widget.first_child();
    while let Some(widget) = child {
        result.extend(descendants(&widget));
        child = widget.next_sibling();
    }
    result
}

fn click(window: &gtk::Window, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    let button = descendants(window.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Button>().ok())
        .find(|button| {
            button
                .child()
                .and_downcast::<gtk::Label>()
                .is_some_and(|label| label.text() == caption)
        })
        .ok_or("button missing")?;
    button.grab_focus();
    button.emit_clicked();
    Ok(())
}

#[test]
#[ignore = "Requires a native GTK display; run explicitly with --ignored --test-threads=1"]
fn native_scene_draft_round_trip_and_focus_recovery() -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let app = gtk::Application::builder()
        .application_id(
            std::env::var("RECITE_BAKEOFF_CAPTURE_APP_ID")
                .unwrap_or_else(|_| "org.recite.BakeoffTest".into()),
        )
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(None::<&gtk::gio::Cancellable>)?;
    recite_bakeoff_gtk::build(&app);
    let window = app.windows().into_iter().next().ok_or("window missing")?;
    capture(&window, "script-light.png")?;
    click(&window, "Dark")?;
    capture(&window, "script-dark.png")?;
    click(&window, "Source")?;
    capture(&window, "source-dark.png")?;
    click(&window, "Light")?;
    capture(&window, "source-light.png")?;
    click(&window, "Script")?;
    let editor = descendants(window.upcast_ref())
        .into_iter()
        .find_map(|widget| widget.downcast::<gtk::TextView>().ok())
        .ok_or("editor missing")?;
    editor.buffer().set_text("A different question.");
    click(&window, "Source")?;
    assert!(descendants(window.upcast_ref()).iter().any(|widget| {
        widget
            .downcast_ref::<gtk::Label>()
            .is_some_and(|label| label.text().contains("Apply or discard"))
    }));
    click(&window, "Apply draft")?;
    click(&window, "Source")?;
    let buffer = editor.buffer();
    let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    assert!(source.contains("A different question."));
    assert!(source.contains("7701ceab59d2adfa057a"));
    click(&window, "Script")?;
    assert_eq!(
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), true),
        "A different question."
    );
    click(&window, "Edit cheshire cat")?;
    assert!(editor.is_focus());
    assert!(
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .contains("That depends")
    );
    click(&window, "Dark")?;
    assert!(window.has_css_class("dark"));
    window.close();
    Ok(())
}

fn capture(window: &gtk::Window, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(directory) = std::env::var("RECITE_BAKEOFF_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::create_dir_all(&directory)?;
    let context = gtk::glib::MainContext::default();
    for _ in 0..24 {
        for _ in 0..100 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let root = window.child().ok_or("window child missing")?;
    if (window.width(), window.height()) != (1200, 800) {
        return Err(format!(
            "Capture requires a 1200 × 800 floating window; got {} × {}. Use scripts/capture.py.",
            window.width(),
            window.height()
        )
        .into());
    }
    let content = gtk::Snapshot::new();
    window.snapshot_child(&root, &content);
    let content = content.to_node().ok_or("empty widget snapshot")?;
    let snapshot = gtk::Snapshot::new();
    let canvas = gtk::gdk::RGBA::parse(if window.has_css_class("dark") {
        "#202124"
    } else {
        "#f5f2ed"
    })?;
    snapshot.append_color(&canvas, &gtk::graphene::Rect::new(0., 0., 1200., 800.));
    snapshot.append_node(&content);
    let node = snapshot.to_node().ok_or("empty widget snapshot")?;
    window
        .renderer()
        .ok_or("native renderer missing")?
        .render_texture(&node, None)
        .save_to_png(std::path::Path::new(&directory).join(name))?;
    Ok(())
}
