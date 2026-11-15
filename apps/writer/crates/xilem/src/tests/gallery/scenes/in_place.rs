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
}
