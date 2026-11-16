//! States of the in-place flows: slips, banners and edits on the object itself.
use super::*;
use crate::workspaces::rename::inline::{Place, Target};
use crate::workspaces::tests::{ToolDriver, finish_build};

pub(in crate::tests::gallery) fn in_place(record: &mut Recorder, variant: Variant) {
    record.scene(
        Meta {
            id: "preview.configure",
            group: "Workspaces",
            title: "Configure preview slip",
            intent: "The next run's inputs on an anchored slip over the running preview, which stays in place beneath it.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            visit(&mut stage, located(Screen::Preview, "relay_hub.recite"));
            stage.act(|app| {
                if let Some(writer) = &mut app.writer {
                    writer.configure_trial();
                }
            });
            stage.shot(record, "Configure preview");
        },
    );
    record.scene(
        Meta {
            id: "script.disk-changed",
            group: "Workspaces",
            title: "Disk change banner",
            intent: "A change made outside Writer, as a banner at the top of the sheet with Compare, Keep mine and Use disk version.",
        },
        |record| {
            let directory = sample();
            let path = directory.path().join("relay_hub.recite");
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            let source = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, source.replacen("Mara", "Mara Vey", 1)).unwrap();
            stage.act(|app| {
                if let Some(writer) = &mut app.writer {
                    let evidence = writer
                        .session
                        .project()
                        .unwrap()
                        .files()
                        .external_check(&path)
                        .unwrap()
                        .observe();
                    writer.notices.found.insert(
                        crate::notices::Target::Source(path.clone()),
                        crate::notices::Evidence::Source(evidence),
                    );
                }
            });
            stage.shot(record, "Changed on disk");
        },
    );
    record.scene(
        Meta {
            id: "script.build-result",
            group: "Workspaces",
            title: "Build result slip",
            intent: "Build as a command: the writing view stays, and the result waits on a slip above the status line; a failure offers its report.",
        },
        |record| {
            let directory = sample();
            let manifest = directory.path().join("recite.project.toml");
            let sample_manifest = std::fs::read_to_string(&manifest).unwrap();
            let declared = sample_manifest.clone()
                + "[[scenes]]\nid = \"relay_desk\"\nasset = \"build/relay_desk.recitec\"\nblock = \"relay_desk\"\nparticipants = [\"mara\"]\n";
            std::fs::write(&manifest, declared).unwrap();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            let build = |app: &mut App| {
                if let Some(writer) = &mut app.writer {
                    writer.build_scenes();
                    finish_build(writer);
                }
            };
            stage.act(build);
            stage.shot(record, "Built");
            // A manifest without scenes fails the same way on every machine.
            std::fs::write(&manifest, sample_manifest).unwrap();
            stage.act(build);
            stage.shot(record, "Build did not complete");
        },
    );
    record.scene(
        Meta {
            id: "script.rename",
            group: "Script",
            title: "Inline rename",
            intent: "Rename beat turns the selected beat's heading, or F2 or a double-click its sidebar row, into a field; Enter reviews the rename, and a slip under the field says what it changes before Rename stages it.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            rename_in(&mut stage, Place::Heading, false);
            stage.shot(record, "Renaming in place");
            rename_in(&mut stage, Place::Heading, true);
            stage.shot(record, "What the rename changes");
            rename_in(&mut stage, Place::Outline, false);
            stage.shot(record, "Renaming in the sidebar");
            rename_in(&mut stage, Place::Outline, true);
            stage.shot(record, "What the rename changes, from the sidebar");
        },
    );
    record.scene(
        Meta {
            id: "map.rename",
            group: "Map",
            title: "Inline rename on a card",
            intent: "F2 or a double-click turns a card's title into a field; the consequence slip hangs under the card, inside the map.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            command(&mut stage, WriterCommand::Map);
            rename_in(&mut stage, Place::Card, false);
            stage.shot(record, "Renaming a card");
            rename_in(&mut stage, Place::Card, true);
            stage.shot(record, "What the rename changes");
        },
    );
    record.scene(
        Meta {
            id: "script.choices",
            group: "Script",
            title: "Insertion, speaker and destination slips",
            intent: "After the beat, + Line and + Reply insert after the passage being written; its speaker label or destination is the control whose choice opens as a slip hanging from it, on the current value.",
        },
        |record| {
            let directory = sample();
            let mut stage = stage(
                App::with_writer(open(directory.path(), "relay_hub.recite")),
                variant,
            );
            let speaker = "Change speaker, mara…";
            let corner = stage.corner(speaker);
            stage.activate(speaker);
            stage.act(|app| {
                if let Some(picker) = &mut app.picker {
                    picker.placement = corner;
                }
            });
            stage.shot(record, "Speaker");
            stage.act(|app| app.dismiss_picker());
            stage.act(|app| {
                let Some(writer) = &mut app.writer else {
                    return;
                };
                let reply = writer.reading.rows.iter().find_map(|row| match &row.content {
                    crate::reading::Content::Passage { passage, .. }
                        if matches!(passage.kind, recite_writer_model::PassageKind::Choice { .. }) =>
                    {
                        Some(passage.id.clone())
                    }
                    _ => None,
                });
                if let Some(reply) = reply {
                    writer.change(|model| model.select(recite_writer_model::View::Passage(reply)));
                }
            });
            let destination = "Change destination, Missing Courier…";
            let corner = stage.corner(destination);
            stage.activate(destination);
            stage.act(|app| {
                if let Some(picker) = &mut app.picker {
                    picker.placement = corner;
                }
            });
            stage.shot(record, "Destination");
        },
    );
}

/// Rename the reading beat in place at `place`, typing a new name, and once `reviewed` let the
/// review finish so its slip shows; any rename already open is cancelled first.
fn rename_in<V: WidgetView<App, Widget = Passthrough>>(
    stage: &mut Stage<App, V>,
    place: Place,
    reviewed: bool,
) {
    stage.act(|app| {
        if let Some(writer) = &mut app.writer {
            writer.cancel_inline_rename();
            let beat = match place {
                Place::Card => writer.map_selected().map(str::to_owned),
                _ => writer.reading.selected.clone(),
            };
            let Some(beat) = beat else {
                return;
            };
            let mut tools = ToolDriver::new(writer);
            writer.rename_beat_in(Target { beat, place });
            writer.edit_inline_rename("front_desk".into());
            if reviewed {
                writer.commit_inline_rename();
                tools.finish(writer);
            }
        }
    });
}
