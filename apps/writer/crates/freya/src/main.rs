use freya::prelude::*;

fn main() {
    let startup = match recite_writer::Startup::parse(std::env::args().skip(1)) {
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}\n{}", recite_writer::startup_help());
            std::process::exit(2);
        }
    };
    if startup == recite_writer::Startup::Help {
        println!("{}", recite_writer::startup_help());
        return;
    }
    if startup == recite_writer::Startup::Version {
        println!("recite-writer {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let specimen = startup == recite_writer::Startup::DesignSystem;
    let owner = match &startup {
        recite_writer::Startup::Writer { project, route } => {
            match recite_writer::Activation::claim(project.as_deref(), route.as_deref()) {
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
                let (file_backed, project, initial_route) = match &startup {
                    recite_writer::Startup::Writer { project, route } => {
                        (true, project.clone(), route.clone())
                    }
                    recite_writer::Startup::Examples { route } => (false, None, route.clone()),
                    _ => (false, None, None),
                };
                if let Some(route) = initial_route {
                    use_provide_context(move || recite_writer::InitialRoute(route));
                }
                if let Some(inbox) = inbox.clone() {
                    use_provide_context(move || inbox);
                }
                use_provide_context(move || recite_writer::InitialProject(project));
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
/// live in `fonts/`, described in `fonts/FONTS.md`.
trait EmbeddedFonts {
    fn with_fonts(self) -> Self;
}

impl EmbeddedFonts for LaunchConfig {
    fn with_fonts(self) -> Self {
        [
            (
                "Fraunces 72pt Soft",
                &include_bytes!("../fonts/fraunces/Fraunces72ptSoft-SemiBold.ttf")[..],
            ),
            (
                "Fraunces",
                &include_bytes!("../fonts/fraunces/Fraunces-Variable.ttf")[..],
            ),
            (
                "Literata",
                &include_bytes!("../fonts/literata/Literata-Variable.ttf")[..],
            ),
            (
                "Literata",
                &include_bytes!("../fonts/literata/Literata-Italic-Variable.ttf")[..],
            ),
            (
                "IBM Plex Sans",
                &include_bytes!("../fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf")[..],
            ),
            (
                "IBM Plex Sans",
                &include_bytes!("../fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf")[..],
            ),
            (
                "IBM Plex Sans",
                &include_bytes!("../fonts/ibm-plex-sans/IBMPlexSans-Medium.ttf")[..],
            ),
            (
                "IBM Plex Sans",
                &include_bytes!("../fonts/ibm-plex-sans/IBMPlexSans-MediumItalic.ttf")[..],
            ),
            (
                "IBM Plex Sans",
                &include_bytes!("../fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf")[..],
            ),
            (
                "IBM Plex Mono",
                &include_bytes!("../fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf")[..],
            ),
            (
                "IBM Plex Mono",
                &include_bytes!("../fonts/ibm-plex-mono/IBMPlexMono-Italic.ttf")[..],
            ),
            (
                "IBM Plex Mono",
                &include_bytes!("../fonts/ibm-plex-mono/IBMPlexMono-Medium.ttf")[..],
            ),
        ]
        .into_iter()
        .fold(self, |config, (family, bytes)| {
            config.with_font(family, bytes)
        })
    }
}
