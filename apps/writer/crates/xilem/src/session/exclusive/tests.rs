#![allow(unused_imports)]
use super::super::*;
use super::*;
use crate::tests::support;
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::FrozenWorkbench;
use recite_writer_session::{FileError, ProjectSession};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{accesskit, core::Widget, testing::TestHarness},
};

fn manuscript<W: Widget>(harness: &TestHarness<W>) -> (accesskit::NodeId, bool) {
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.label().as_deref() == Some("Manuscript")
            && node.role() == accesskit::Role::MultilineTextInput
        {
            return (node.locate().0, node.is_disabled());
        }
        pending.extend(node.children());
    }
    panic!("mounted manuscript");
}

#[test]
fn checkout_disables_the_mounted_native_manuscript_and_return_reenables_the_same_field() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = crate::view::writer(&writer);
    let (pod, mut state) = view.build(&mut context, &mut writer);
    let mut harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        pod.new_widget,
        (1280, 800),
    );
    let (original, disabled) = manuscript(&harness);
    assert!(!disabled);
    let mut checkout = writer.session.checkout().unwrap();
    let next = crate::view::writer(&writer);
    harness
        .edit_root_widget(|root| next.rebuild(&view, &mut state, &mut context, root, &mut writer));
    view = next;
    harness.redraw();
    assert_eq!(manuscript(&harness), (original, true));
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut operation = false;
    while let Some(node) = pending.pop() {
        assert_ne!(node.label().as_deref(), Some("Temporary examples"));
        assert_ne!(node.value().as_deref(), Some("Temporary examples"));
        operation |= node.label().as_deref() == Some("Project operation in progress")
            || node.value().as_deref() == Some("Project operation in progress");
        pending.extend(node.children());
    }
    assert!(operation);
    writer
        .session
        .restore_checkout(&mut checkout, None)
        .unwrap();
    let next = crate::view::writer(&writer);
    harness
        .edit_root_widget(|root| next.rebuild(&view, &mut state, &mut context, root, &mut writer));
    harness.redraw();
    assert_eq!(manuscript(&harness), (original, false));
}

#[test]
fn checkout_leaves_only_reading_authority_and_restores_the_same_lease_and_edit_history() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let original = writer.session.model().document().source().to_owned();
    writer.change(|model| {
        model.set_draft("Accepted edit.".into());
        model.apply()
    });
    let source = writer.session.model().document().source_snapshot();
    let mut checkout = writer.session.checkout().unwrap();
    assert!(writer.session.project_mut().is_none());
    assert!(writer.session.parts_mut().is_none());
    assert!(writer.session.read_only());
    assert!(Arc::ptr_eq(
        &source,
        &writer.session.model().document().source_snapshot()
    ));
    assert!(!writer.session.model().document().can_undo());
    assert!(
        writer
            .session
            .edit(|_| panic!("queued edit must not run"))
            .is_err()
    );
    assert!(matches!(
        writer.session.advance_preview(None),
        Err(FileError::ProjectActionPending)
    ));
    assert!(
        writer
            .begin_navigation(crate::navigation::Target::Edit(
                recite_writer_model::EditOperation::Undo
            ))
            .is_err()
    );
    writer.request_close();
    assert!(writer.phase == crate::Phase::Writing);
    assert!(matches!(
        ProjectSession::open(directory.path(), None),
        Err(FileError::RecoveryInUse)
    ));
    writer
        .session
        .restore_checkout(&mut checkout, None)
        .unwrap();
    assert!(!writer.session.read_only());
    assert!(writer.session.model().document().can_undo());
    assert!(checkout.project.is_none());
    assert!(checkout.retired.is_some());
    writer.change(|model| model.undo());
    assert_eq!(writer.session.model().document().source(), original);
}
#[test]
fn foreign_and_repeated_returns_cannot_replace_the_real_project_owner() {
    let first = support::project(recite_writer_model::FIXTURE).unwrap();
    let second = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut a = support::open(&first).unwrap();
    let mut b = support::open(&second).unwrap();
    let mut own = a.session.checkout().unwrap();
    let mut foreign = b.session.checkout().unwrap();
    assert!(a.session.restore_checkout(&mut foreign, None).is_err());
    assert!(a.session.checked_out());
    assert!(foreign.project.is_some());
    a.session.restore_checkout(&mut own, None).unwrap();
    assert!(a.session.restore_checkout(&mut own, None).is_err());
    assert_eq!(a.session.project().unwrap().root(), first.path());
    b.session.restore_checkout(&mut foreign, None).unwrap();
    assert_eq!(b.session.project().unwrap().root(), second.path());
}
