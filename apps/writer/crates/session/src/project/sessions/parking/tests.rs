use super::two_scenes;
use crate::recovery::RecoveryStore;
use recite_compiler::authoring::{Interrupted, WorkControl};
use recite_writer_model::{EditError, View};
use std::sync::atomic::{AtomicUsize, Ordering};

struct AfterFirst(AtomicUsize);
impl WorkControl for AfterFirst {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                left.checked_sub(1)
            })
            .map(drop)
            .map_err(|_| Interrupted)
    }
}

#[test]
fn failed_or_cancelled_navigation_restarts_the_original_recovery_owner()
-> Result<(), Box<dyn std::error::Error>> {
    for cancel in [false, true] {
        let (_dir, mut files, mut current, other) = two_scenes()?;
        let original = files.current.clone();
        current.select(View::Source)?;
        let accepted = format!("{}\n# Accepted before navigation\n", current.draft());
        current.set_draft(accepted.clone());
        current.apply()?;
        let result = if cancel {
            files.switch_with_control(
                &mut current,
                &other,
                None,
                |_| Ok(()),
                &AfterFirst(AtomicUsize::new(1)),
            )
        } else {
            files.switch(&mut current, &other, None, |_| {
                Err(EditError::Destination.into())
            })
        };
        assert!(result.is_err());
        assert_eq!(files.current, original);
        assert_eq!(current.document().source(), accepted);
        current.set_draft("A new draft after failed navigation".into());
        files.queue_checkpoint(&current)?;
        files.checkpoint(&current)?;
        drop(files);
        let reopened = RecoveryStore::open(&original)?;
        let snapshot = reopened.snapshot().ok_or("recovery snapshot")?;
        assert_eq!(snapshot.draft.source(), accepted);
        assert_eq!(
            snapshot.draft.draft_snapshot().as_str(),
            "A new draft after failed navigation"
        );
    }
    Ok(())
}
