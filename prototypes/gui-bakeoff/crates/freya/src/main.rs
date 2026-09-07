use freya::prelude::*;

fn main() {
    let file_backed = std::env::args().nth(1).as_deref() == Some("--project");
    // Freya keeps a static title for the process-wide event loop.
    let title = std::env::var("RECITE_BAKEOFF_WINDOW_ID")
        .map(|title| &*Box::leak(title.into_boxed_str()))
        .unwrap_or(if file_backed {
            "Recite writer"
        } else {
            "Recite — Freya bake-off"
        });

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(move || {
                if file_backed {
                    recite_bakeoff_freya::editor_app()
                } else {
                    recite_bakeoff_freya::app()
                }
            })
            .with_title(title)
            .with_size(1200., 800.),
        ),
    );
}
