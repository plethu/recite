//! States of the in-place flows: slips, banners and edits on the object itself.
use super::*;
use crate::workspaces::tests::finish_build;

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
}
