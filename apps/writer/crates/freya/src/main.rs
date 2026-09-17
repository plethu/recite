use freya::prelude::*;

fn main() {
    let file_backed = std::env::args().any(|arg| arg == "--project");
    let args: Vec<_> = std::env::args().collect();
    let initial_route = args
        .windows(2)
        .find(|pair| pair[0] == "--route")
        .map(|pair| pair[1].clone());
    let title = if file_backed {
        "Recite writer"
    } else {
        "Recite — Writer examples"
    };

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(move || {
                // Command-line arguments are fixed for this window's lifetime.
                if let Some(route) = initial_route.clone() {
                    use_provide_context(move || recite_writer::InitialRoute(route));
                }
                let _preferences = use_provide_context(|| {
                    recite_config::UserConfigStore::discover().map_err(|e| e.to_string())
                });
                let _storage = use_provide_context(|| {
                    recite_config::UserConfigStore::discover()
                        .map_err(|e| e.to_string())
                        .and_then(|store| {
                            store
                                .state_file("writer-layouts.json")
                                .map_err(|e| e.to_string())
                        })
                });
                if file_backed {
                    recite_writer::editor_app()
                } else {
                    recite_writer::app()
                }
            })
            .with_on_close(|_, _| recite_writer::request_close())
            .with_title(title)
            .with_size(1200., 800.)
            .with_min_size(900., 650.),
        ),
    );
}
