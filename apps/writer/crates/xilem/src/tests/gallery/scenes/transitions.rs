//! Frame sequences through interactions, one frame per step and optional motion frames.
use super::*;

pub(in crate::tests::gallery) fn transitions(
    record: &mut Recorder,
    variant: Variant,
    settings: &Settings,
) {
    record.scene(
        Meta {
            id: "flow.examples",
            group: "Transitions",
            title: "Welcome to examples",
            intent: "Arrival: from the welcome screen into temporary examples.",
        },
        |record| {
            let mut stage = stage(App::empty(), variant);
            step(&mut stage, record, settings, "Welcome");
            stage.hover("Try temporary examples");
            step(&mut stage, record, settings, "Hovering the examples action");
            stage.park_pointer();
            stage.activate("Try temporary examples");
            step(&mut stage, record, settings, "Examples open");
        },
    );
    record.scene(
        Meta {
            id: "flow.workspaces",
            group: "Transitions",
            title: "Switching workspaces",
            intent: "Script to Map to Split and back: continuity of place and orientation.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            step(&mut stage, record, settings, "Script");
            for (caption, next) in [
                ("Map", WriterCommand::Map),
                ("Split", WriterCommand::Split),
                ("Back to script", WriterCommand::Script),
            ] {
                command(&mut stage, next);
                step(&mut stage, record, settings, caption);
            }
        },
    );
    record.scene(
        Meta {
            id: "flow.commands",
            group: "Transitions",
            title: "Running a command",
            intent: "Open the command picker, filter it, and return: feedback at each step.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            step(&mut stage, record, settings, "Before");
            stage.act(|app| app.open_picker(Search::Commands));
            step(&mut stage, record, settings, "Picker open");
            stage.paste("Search query", "map");
            step(&mut stage, record, settings, "Typed \"map\"");
            stage.act(|app| app.submit_picker());
            step(&mut stage, record, settings, "Command ran");
        },
    );
    record.scene(
        Meta {
            id: "flow.close",
            group: "Transitions",
            title: "Closing with unsaved work",
            intent: "Request close, see the decision, keep editing: focus lands safely.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            draft(&mut stage, "Unsaved line.");
            step(&mut stage, record, settings, "Editing");
            stage.act(|app| app.request_close());
            step(&mut stage, record, settings, "Close requested");
            stage.activate("Keep editing");
            step(&mut stage, record, settings, "Kept editing");
        },
    );
    record.scene(
        Meta {
            id: "flow.controls",
            group: "Transitions",
            title: "Control states",
            intent: "Rest, hover, pressed and keyboard focus on app-bar controls: tactility and focus visibility.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(App::with_writer(open(directory.path(), "relay_hub.recite")), variant);
            step(&mut stage, record, settings, "Rest");
            if stage.hover("Commands") {
                step(&mut stage, record, settings, "Hover");
            }
            if stage.press("Commands") {
                step(&mut stage, record, settings, "Pressed");
                stage.release();
            }
            stage.park_pointer();
            stage.act(|app| app.dismiss_picker());
            if stage.focus("Commands") {
                step(&mut stage, record, settings, "Keyboard focus");
            }
        },
    );
    record.scene(
        Meta {
            id: "flow.tab-walk",
            group: "Transitions",
            title: "Keyboard traversal",
            intent: "The first Tab stops from the manuscript: order, visibility and focus ring contrast.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(App::with_writer(open(directory.path(), "relay_hub.recite")), variant);
            stage.harness.focus_on(None);
            for index in 1..=8 {
                stage.tab(false);
                stage.shot(record, &format!("Tab {index}"));
            }
        },
    );
    record.scene(
        Meta {
            id: "flow.preview",
            group: "Transitions",
            title: "Previewing a scene",
            intent: "Start a trial and take the first reply: pacing and clarity of choice.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            step(&mut stage, record, settings, "Script");
            command(&mut stage, WriterCommand::Preview);
            step(&mut stage, record, settings, "Preview open");
            stage.activate("Continue preview");
            step(&mut stage, record, settings, "Replies offered");
            if stage.activate_containing("Anything I can do") {
                step(&mut stage, record, settings, "First reply taken");
            }
        },
    );
}
