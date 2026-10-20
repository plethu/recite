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
