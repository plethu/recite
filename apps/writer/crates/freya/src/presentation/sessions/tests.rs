use super::*;
use recite_writer_model::Document;
use std::sync::Arc;

#[test]
fn closed_scene_rules_retire_even_when_the_worker_receipt_is_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n? reply@11111111111111111111\n  Continue.\n  -> END\n";
    let document = Document::new(source)?;
    let rules = document.reply_rules("11111111111111111111")?;
    let owner = document.source_snapshot();
    let weak = Arc::downgrade(&owner);
    drop(owner);
    drop(document);
    assert_eq!(weak.strong_count(), 1);
    let retired = RetiredPresentation {
        positions: BTreeMap::new(),
        rules: Some(rules),
    };
    crate::editor_work::worker::Worker::retire_presentation(retired)?;
    for _ in 0..5_000 {
        if weak.strong_count() == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(weak.upgrade().is_none());
    Ok(())
}
