use super::*;
use crate::{
    Writer, external_editor::tests::Driver as EditorDriver, keyed::keyed,
    tests::support::mounted::Mounted, workspaces::tests::ToolDriver,
};
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_merge_pages_preserve_drafts_through_rejected_import_and_save_the_reviewed_repair() {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let draft = format!(
        "Pending start {} Pending end",
        "retained author prose ".repeat(100)
    );
    writer.change(|model| {
        model.set_draft(draft.clone());
        Ok(())
    });
    let path = directory.path().join("scene.recite");
    let external = SOURCE.replace("Original manuscript.", "External manuscript.");
    fs::write(&path, &external).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    let mut tools = ToolDriver::new(&mut mounted.state);
    let mut editor = EditorDriver::new(&mut mounted.state);
    mounted.state.compare_disk_changes();
    tools.finish(&mut mounted.state);
    mounted.settle();
    assert!(mounted.contains_text("Pending start"));
    mounted.activate("More field draft");
    assert!(!mounted.contains_text("Pending start"));
    mounted.activate("Previous field draft part");
    assert!(mounted.contains_text("Pending start"));
    mounted.activate("Export merge draft");
    tools.finish(&mut mounted.state);
    mounted.settle();
    let artifact = mounted
        .state
        .workspaces
        .external
        .inspection()
        .unwrap()
        .artifact
        .as_ref()
        .unwrap()
        .source
        .clone();
    assert!(fs::read_to_string(&artifact).unwrap().contains(&draft));
    mounted.activate("Open merge draft in editor");
    editor.complete(&mut mounted.state, &artifact, Ok(()));
    assert!(mounted.state.status.contains("Import reviewed merge"));
    fs::write(
        &artifact,
        format!(
            ":: start default\n-> missing_{}\n",
            "long_target_".repeat(200)
        ),
    )
    .unwrap();
    mounted.activate("Import reviewed merge");
    tools.finish(&mut mounted.state);
    mounted.settle();
    assert_eq!(mounted.state.session.model().draft(), draft);
    assert_eq!(mounted.state.session.model().document().source(), SOURCE);
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert!(
        mounted.state.workspaces.external.report.len() > 1,
        "complete validation details must remain available: {}",
        mounted.state.status
    );
    mounted.activate("More comparison report");
    mounted.activate("Previous comparison report");
    let merged = SOURCE.replace("Original manuscript.", "Reviewed merged manuscript.");
    fs::write(&artifact, &merged).unwrap();
    mounted.activate("Import reviewed merge");
    tools.finish(&mut mounted.state);
    mounted.settle();
    assert_eq!(mounted.state.session.model().document().source(), merged);
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert!(mounted.state.recovery_copy.as_ref().unwrap().is_file());
    assert!(mounted.contains_text("Reviewed merge adopted"));
    assert!(!mounted.contains_text("Keep Writer version"));
    assert!(!mounted.contains_text("Import reviewed merge"));
    mounted.activate("Return to writing");
    let (sender, mut saves) = mpsc::unbounded_channel();
    mounted.state.work.sender = Some(sender);
    mounted.activate("Save");
    support::finish_work(&mut mounted.state, &mut saves).unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), merged);
}

#[test]
fn completed_disk_decisions_replace_unresolved_choices_at_both_scales() {
    for scale in [100, 200] {
        for (keep, heading) in [
            (true, "Writer version kept"),
            (false, "Inspected disk version loaded"),
        ] {
            let directory = support::project(SOURCE).unwrap();
            let mut writer = support::open(&directory).unwrap();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            let path = directory.path().join("scene.recite");
            let disk = SOURCE.replace("Original manuscript.", "External manuscript.");
            fs::write(&path, &disk).unwrap();
            let mut mounted = Mounted::new(writer, surface);
            let mut tools = ToolDriver::new(&mut mounted.state);
            mounted.state.compare_disk_changes();
            tools.finish(&mut mounted.state);
            mounted.settle();
            if keep {
                mounted.state.keep_writer_version();
            } else {
                mounted.state.use_inspected_disk();
            }
            tools.finish(&mut mounted.state);
            mounted.settle();
            assert!(mounted.contains_text(heading));
            assert!(mounted.contains_text("Return to writing"));
            assert!(mounted.contains_text("Show recovery copy location"));
            for retired in [
                "Compare disk changes",
                "Keep Writer version",
                "Use inspected disk version",
                "Export merge draft",
            ] {
                assert!(!mounted.contains_text(retired), "{retired}");
            }
            assert_eq!(fs::read_to_string(path).unwrap(), disk);
            mounted.activate("Inspect current disk version");
            tools.finish(&mut mounted.state);
            mounted.settle();
            assert!(mounted.contains_text("Compare disk changes"));
            assert!(!mounted.contains_text(heading));
        }
    }
}
