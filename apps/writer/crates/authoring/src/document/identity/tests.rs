use super::*;
use crate::EditError;

#[test]
fn stamp_tracks_analysis_replacement_without_retaining_project_payloads() -> Result<(), EditError> {
    let mut document = Document::new(":: start default\n-> END\n")?;
    let stamp = document.analysis_stamp();
    assert!(stamp.is_current(&document));
    assert_eq!(Arc::strong_count(&document.kernel), 1);
    assert_eq!(Arc::strong_count(&document.context), 1);
    let replacement = Document::new(document.source())?;
    assert_eq!(replacement.revision(), document.revision());
    assert!(!stamp.is_current(&replacement));
    document.refresh_project(ProjectContext::default())?;
    assert!(
        !stamp.is_current(&document),
        "context identity is significant"
    );
    assert!(stamp.context.upgrade().is_none());
    assert!(stamp.kernel.upgrade().is_some());
    drop(document);
    assert!(stamp.kernel.upgrade().is_none());
    assert!(stamp.source.upgrade().is_none());
    Ok(())
}

#[test]
fn navigation_stamp_does_not_keep_compiler_or_context_payloads_alive()
-> Result<(), crate::WorkbenchError> {
    let document = Document::new(":: start default\n-> END\n")?;
    let kernel = Arc::downgrade(&document.kernel);
    let context = Arc::downgrade(&document.context);
    let model = crate::Workbench::from_document(document, Some(crate::View::Source))?;
    let kernel_owners = kernel.strong_count();
    let context_owners = context.strong_count();
    let stamp = model.navigation_stamp();
    assert!(stamp.is_current(&model));
    assert_eq!(kernel.strong_count(), kernel_owners);
    assert_eq!(context.strong_count(), context_owners);
    drop(model);
    assert!(kernel.upgrade().is_none());
    assert!(context.upgrade().is_none());
    drop(stamp);
    Ok(())
}
