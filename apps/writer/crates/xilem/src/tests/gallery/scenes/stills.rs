//! One frame per state: empty and representative data on every surface.
use super::*;

pub(in crate::tests::gallery) fn stills(record: &mut Recorder, variant: Variant) {
    record.scene(
        Meta {
            id: "start.welcome",
            group: "Start",
            title: "Welcome, no project",
            intent: "First launch: the only surface before any project exists. Judge invitation, hierarchy and the print identity.",
        },
        |record| stage(App::empty(), variant).shot(record, "Welcome"),
    );
    record.scene(
        Meta {
            id: "start.create",
            group: "Start",
            title: "Welcome, creating a project",
            intent: "The create form with nothing typed yet: field labels, guidance and the disabled primary action.",
        },
        |record| {
            let mut stage = stage(App::empty(), variant);
            stage.act(|app| app.choose_project_action(true));
            stage.shot(record, "Create form, empty");
        },
    );
    record.scene(
        Meta {
            id: "script.starter",
            group: "Script",
            title: "Script, new starter project",
            intent: "Empty state a new writer meets right after creating a project.",
        },
        |record| {
            let directory = starter();
            let writer = open(&directory.path().join("harbour"), "");
            stage(App::with_writer(writer), variant).shot(record, "Starter scene");
        },
    );
    record.scene(
        Meta {
            id: "script.empty-beat",
            group: "Script",
            title: "Script, beat with nothing written",
            intent: "Empty beat placeholder and the invitation to write the first line.",
        },
        |record| {
            stage(App::with_writer(examples("empty_beat.recite")), variant)
                .shot(record, "Empty beat");
        },
    );
    record.scene(
        Meta {
            id: "script.blank-scene",
            group: "Script",
            title: "Script, scene with no beats",
            intent: "A scene that is not valid yet: how the problem and its fix are presented.",
        },
        |record| {
            stage(App::with_writer(examples("blank_scene.recite")), variant)
                .shot(record, "Blank scene");
        },
    );
    record.scene(
        Meta {
            id: "script.sample",
            group: "Script",
            title: "Script, representative scene",
            intent: "Everyday reading and writing in a branching hub: speakers, replies, destinations, effects.",
        },
        |record| {
            let directory = sample();
            stage(App::with_writer(open(directory.path(), "relay_hub.recite")), variant)
                .shot(record, "Relay hub beat");
        },
    );
    record.scene(
        Meta {
            id: "script.draft",
            group: "Script",
            title: "Script, unapplied draft",
            intent: "Dirty state: how pending, unsaved work is signalled without alarm.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            draft(
                &mut stage,
                "If you're here about the transmitter, it's fixed. Mostly.",
            );
            stage.shot(record, "Draft pending");
        },
    );
    record.scene(
        Meta {
            id: "script.focus",
            group: "Script",
            title: "Script, focus mode",
            intent: "Long-session writing with chrome reduced.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            command(&mut stage, WriterCommand::Focus);
            stage.shot(record, "Focus mode");
        },
    );
    record.scene(
        Meta {
            id: "map.sample",
            group: "Map",
            title: "Map, branching scene",
            intent: "How beats branch and return; card legibility and connection clarity.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "floodgate_waterfall.recite")),
                variant,
            );
            command(&mut stage, WriterCommand::Map);
            stage.shot(record, "Floodgate map");
        },
    );
    record.scene(
        Meta {
            id: "map.split",
            group: "Map",
            title: "Split, script beside map",
            intent: "Two panes sharing the window: balance, divider and which pane leads.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            command(&mut stage, WriterCommand::Split);
            stage.shot(record, "Split workspace");
        },
    );
    record.scene(
        Meta {
            id: "source.repair",
            group: "Script",
            title: "Source needing repair",
            intent: "Malformed source: Writer hands repair to the external editor. Judge clarity and calm.",
        },
        |record| {
            let directory = sample();
            fs::write(directory.path().join("broken.recite"), ":: broken default\n->\n").expect("broken scene");
            stage(App::with_writer(open(directory.path(), "broken.recite")), variant)
                .shot(record, "Repair in your editor");
        },
    );
    for (id, title, intent, screen, document) in [
        (
            "preview.start",
            "Preview, before a run",
            "A trial of the scene that never runs game effects.",
            Screen::Preview,
            "relay_hub.recite",
        ),
        (
            "localise.beat",
            "Localise, beat with a catalogue",
            "Side-by-side translation with translated, variant and missing entries.",
            Screen::Localise,
            "harbour.recite",
        ),
        (
            "localise.queue",
            "Translations queue",
            "The catalogue work queue and its filters.",
            Screen::Translations,
            "harbour.recite",
        ),
        (
            "tools.build",
            "Build",
            "Producing game data and reporting the result.",
            Screen::Build,
            "relay_hub.recite",
        ),
        (
            "tools.rename",
            "Rename",
            "Stable-ID-preserving rename with comparison.",
            Screen::Rename,
            "relay_hub.recite",
        ),
        (
            "tools.declarations",
            "Declarations",
            "Schema speakers, effects and variables.",
            Screen::Declarations,
            "relay_hub.recite",
        ),
        (
            "tools.disk",
            "Disk changes",
            "Changes made outside Writer, offered for explicit comparison.",
            Screen::Disk,
            "relay_hub.recite",
        ),
        (
            "tools.updates",
            "Source updates",
            "Catalogue entries whose source text changed.",
            Screen::Updates,
            "harbour.recite",
        ),
    ] {
        record.scene(
            Meta {
                id,
                group: "Workspaces",
                title,
                intent,
            },
            |record| {
                let directory = sample();
                let mut stage = stage(App::with_writer(open(directory.path(), document)), variant);
                let mut location = located(screen, document);
                if matches!(
                    screen,
                    Screen::Localise | Screen::Translations | Screen::Updates
                ) {
                    location.catalogue = Some("fr.po".into());
                }
                visit(&mut stage, location);
                stage.shot(record, title);
            },
        );
    }
    record.scene(
        Meta {
            id: "localise.none",
            group: "Workspaces",
            title: "Localise, no catalogue yet",
            intent: "Empty state: a project with nothing to translate into yet.",
        },
        |record| {
            let directory = starter();
            let mut stage = stage(
                App::with_writer(open(&directory.path().join("harbour"), "")),
                variant,
            );
            command(&mut stage, WriterCommand::Localise);
            stage.shot(record, "No catalogue");
        },
    );
    record.scene(
        Meta {
            id: "overlay.settings",
            group: "Overlays",
            title: "Preferences",
            intent: "Personal preferences: appearance, text, motion and shortcuts.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            stage.act(|app| app.settings_from_menu());
            stage.shot(record, "Preferences");
        },
    );
    for (id, title, search) in [
        ("overlay.commands", "Command picker", Search::Commands),
        ("overlay.goto", "Go to beat", Search::GoTo),
        ("overlay.find", "Find in project", Search::ProjectText),
        ("overlay.passages", "Passage lookup", Search::Passages),
    ] {
        record.scene(
            Meta {
                id,
                group: "Overlays",
                title,
                intent: "Keyboard-first search over the open project, before anything is typed.",
            },
            |record| {
                let directory = sample();
                let mut stage = stage(
                    App::with_writer(open(directory.path(), "relay_hub.recite")),
                    variant,
                );
                stage.act(|app| app.open_picker(search));
                stage.shot(record, title);
            },
        );
    }
    record.scene(
        Meta {
            id: "overlay.close",
            group: "Overlays",
            title: "Close with unsaved work",
            intent: "A consequential decision: wording, safe default and button weight.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            draft(&mut stage, "Unsaved line.");
            stage.act(|app| app.request_close());
            stage.shot(record, "Close decision");
        },
    );
}
