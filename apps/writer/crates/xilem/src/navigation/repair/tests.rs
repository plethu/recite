use super::*;
use crate::tests::support::NoProxy;
use tokio::sync::mpsc as async_mpsc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
};

#[test]
fn source_repair_export_preserves_draft_and_disk_then_imports_checked_text() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let (sender, mut launches) = async_mpsc::unbounded_channel();
    writer.external.sender = Some(sender);
    writer.change(|model| {
        model.select(View::Source)?;
        model.stage_repaired_source(":: first default\n? invalid".to_owned().into())
    });
    writer.export_source_repair();
    assert!(writer.navigation.busy());
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    let artifact = writer.repair.artifact.as_ref().unwrap().clone();
    assert_eq!(launches.try_recv().unwrap().path(), artifact.source);
    assert_eq!(std::fs::read_to_string(&artifact.document).unwrap(), SOURCE);
    assert_eq!(
        std::fs::read_to_string(&artifact.source).unwrap(),
        ":: first default\n? invalid"
    );
    let draft = writer.session.model().recovery();
    writer.save();
    assert!(!writer.work.busy());
    assert!(writer.status.contains("Repair and import"));
    writer.import_source_repair();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().recovery(), draft);
    let repaired = SOURCE.replace("First.", "Repaired source.");
    std::fs::write(&artifact.source, &repaired).unwrap();
    writer.import_source_repair();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().document().source(), repaired);
    assert!(!writer.session.model().has_draft());
    assert_eq!(
        std::fs::read_to_string(&artifact.document).unwrap(),
        SOURCE,
        "import requires an explicit Save to write main source"
    );
    assert!(artifact.recovery.is_file());
    writer.navigate(Target::Edit(EditOperation::Undo));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().document().source(), SOURCE);
}

#[test]
fn cancellation_reconciles_export_but_never_launches_or_publishes_an_import() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let (sender, mut launches) = async_mpsc::unbounded_channel();
    writer.external.sender = Some(sender);
    writer.change(|model| {
        model.select(View::Source)?;
        model.stage_repaired_source("Unfinished recovered source.".to_owned().into())
    });
    writer.export_source_repair();
    let completed = fixture.jobs.try_recv().unwrap().execute();
    writer.cancel_navigation();
    drop(writer.accept_navigation(completed));
    let artifact = writer.repair.artifact.as_ref().unwrap().clone();
    assert!(artifact.source.is_file() && artifact.recovery.is_file());
    assert!(launches.try_recv().is_err());
    let repaired = SOURCE.replace("First.", "External repair.");
    std::fs::write(&artifact.source, repaired).unwrap();
    writer.import_source_repair();
    let completed = fixture.jobs.try_recv().unwrap().execute();
    writer.change(|model| {
        model.set_draft("Newer recovered draft.".into());
        Ok(())
    });
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.session.model().draft(), "Newer recovered draft.");
    assert_eq!(writer.session.model().document().source(), SOURCE);
    writer.import_source_repair();
    let completed = fixture.jobs.try_recv().unwrap().execute();
    writer.cancel_navigation();
    drop(writer.accept_navigation(completed));
    assert_eq!(writer.session.model().draft(), "Newer recovered draft.");
    assert_eq!(writer.session.model().document().source(), SOURCE);
}

#[test]
fn legacy_source_recovery_cannot_bypass_import_via_links_or_retained_save_all() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.change(|model| {
        model.select(View::Source)?;
        model.set_draft(":: first default\n? invalid".into());
        Ok(())
    });
    assert_eq!(
        writer.session.model().source_validation(),
        recite_writer_model::SourceValidation::Freeform
    );
    let before = writer.session.model().recovery();
    writer.navigate(Target::Location(
        recite_writer_session::location::Location {
            document: "other.recite".into(),
            ..Default::default()
        },
    ));
    assert!(!writer.navigation.busy());
    writer.navigate(Target::Edit(EditOperation::Apply));
    assert!(!writer.navigation.busy());
    assert_eq!(writer.session.model().recovery(), before);
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    writer.switch(&fixture.other);
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    writer.change(|model| {
        model.set_draft("Unapplied current prose.".into());
        Ok(())
    });
    writer.save_all();
    assert!(!writer.work.busy());
    assert!(writer.status.contains("source draft in scene.recite"));
    assert_eq!(writer.session.model().draft(), "Unapplied current prose.");
    writer.save_and_close();
    assert!(!writer.work.busy());
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    writer.switch(&fixture.directory.path().join("scene.recite"));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().recovery(), before);
}

#[test]
fn repair_controls_expose_the_recovered_source_route_at_personal_scales() {
    for scale in [100, 200] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.change(|model| {
            model.select(View::Source)?;
            model.stage_repaired_source("Recovered source draft.".to_owned().into())
        });
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(recite_config::WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config);
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = crate::view::writer(writer);
        let (pod, _) = view.build(&mut ctx, writer);
        let defaults = crate::presentation::Theme::new(&writer.config.writer).initial_properties();
        let mut harness = TestHarness::create_with_size(
            defaults,
            NewWidget::new(SizedBox::new(pod.new_widget)),
            (1280, 800),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let (mut export, mut input, mut choose) = (false, false, false);
        while let Some(node) = pending.pop() {
            export |= node.label().as_deref() == Some("Export draft and open editor")
                && !node.is_disabled();
            input |= node.label().as_deref() == Some("Repaired source file");
            choose |= node.label().as_deref() == Some("Choose file…") && !node.is_disabled();
            pending.extend(node.children());
        }
        assert!(export && input && choose);
        if let Ok(prefix) = std::env::var("RECITE_CAPTURE_SOURCE_REPAIR") {
            harness
                .render()
                .save(format!("{prefix}-{scale}.png"))
                .unwrap();
        }
    }
}
