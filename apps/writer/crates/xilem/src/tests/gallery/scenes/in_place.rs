//! States of the in-place flows: slips, banners and edits on the object itself.
use super::*;

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
}
