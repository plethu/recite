use freya::prelude::*;

fn main() {
    let options = match recite_writer::LaunchOptions::parse(std::env::args_os().skip(1)) {
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}\n{}", recite_writer::startup_help());
            std::process::exit(2);
        }
    };
    let startup = options.intent.clone();
    if startup == recite_writer::Startup::Help {
        println!("{}", recite_writer::startup_help());
        return;
    }
    if startup == recite_writer::Startup::Version {
        println!("recite-writer {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let initial_route = match options.route() {
        Ok(route) => route,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let specimen = startup == recite_writer::Startup::DesignSystem;
    let owner = match &startup {
        recite_writer::Startup::Writer { .. } => {
            match recite_writer::Activation::claim_launch(&options) {
                Ok(recite_writer::Activation::Owner(owner)) => Some(owner),
                Ok(recite_writer::Activation::Forwarded) => return,
                Err(error) => {
                    eprintln!("Could not activate Recite writer: {error}");
                    std::process::exit(2);
                }
            }
        }
        _ => None,
    };
    let inbox = owner.as_ref().map(recite_writer::ActivationHost::inbox);
    let title = match startup {
        recite_writer::Startup::DesignSystem => "Recite — Component specimen",
        recite_writer::Startup::Examples { .. } => "Recite — Writer examples",
        _ => "Recite writer",
    };

    launch(
        LaunchConfig::new().with_fonts().with_window(
            WindowConfig::new(move || {
                if specimen {
                    return recite_writer::design_app();
                }
                // Startup intent is fixed for this window's lifetime.
                let (file_backed, project) = match &startup {
                    recite_writer::Startup::Writer { project, .. } => (true, project.clone()),
                    _ => (false, None),
                };
                if let Some(route) = initial_route.clone() {
                    use_provide_context(move || recite_writer::InitialRoute(route));
                }
                if let Some(inbox) = inbox.clone() {
                    use_provide_context(move || inbox);
                }
                use_provide_context(move || recite_writer::InitialProject(project));
                let editor = options.editor.clone();
                use_provide_context(move || recite_writer::InitialEditor(editor));
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
            .with_on_close(move |_, _| {
                if specimen {
                    CloseDecision::Close
                } else {
                    recite_writer::request_close()
                }
            })
            .with_title(title)
            .with_app_id("io.github.plethu.recite")
            .with_icon(LaunchConfig::window_icon(include_bytes!(
                "../../../packaging/icons/recite-writer.png"
            )))
            .with_size(1200., 800.)
            .with_min_size(900., 650.),
        ),
    );
}

/// Writer ships its four OFL families so the type chains resolve without system fonts.
/// Each name matches a family in the chains in `design/tokens.rs`; the files and their licences
/// live in `../../assets/fonts/`, described in its `FONTS.md`.
trait EmbeddedFonts {
    fn with_fonts(self) -> Self;
}

impl EmbeddedFonts for LaunchConfig {
    fn with_fonts(self) -> Self {
        recite_writer_session::typography::BUNDLED_FONTS
            .iter()
            .fold(self, |config, (family, bytes)| {
                config.with_font(*family, *bytes)
            })
    }
}
