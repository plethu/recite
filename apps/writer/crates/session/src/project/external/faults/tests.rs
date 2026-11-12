use super::*;
use recite_compiler::authoring::{Interrupted, WorkControl};
use std::sync::atomic::{AtomicBool, Ordering};

enum AfterArchive {
    Cancel,
    ReplaceDisk,
}
struct Control {
    root: PathBuf,
    action: AfterArchive,
    used: AtomicBool,
}
impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        let archived = fs::read_dir(&self.root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".recite-recovered-")
        });
        if archived && !self.used.swap(true, Ordering::Relaxed) {
            match self.action {
                AfterArchive::Cancel => return Err(Interrupted),
                AfterArchive::ReplaceDisk => {
                    fs::write(self.root.join("scene.recite"), "# newer disk\n").unwrap()
                }
            }
        }
        Ok(())
    }
}
#[test]
fn cancellation_and_disk_change_after_archiving_preserve_the_live_owner_and_recoverable_archive() {
    for action in [AfterArchive::Cancel, AfterArchive::ReplaceDisk] {
        let (directory, mut files, mut model) = fixture();
        model.set_draft("Pending local text.".into());
        let before = model.recovery();
        fs::write(&files.current, SOURCE.replace("Original.", "External.")).unwrap();
        files.compare_external(&model).unwrap();
        let control = Control {
            root: directory.path().to_owned(),
            action,
            used: AtomicBool::new(false),
        };
        assert!(
            files
                .resolve_external_with_control(
                    &mut model,
                    ExternalDecision::UseInspectedDisk,
                    &control
                )
                .is_err()
        );
        assert!(control.used.load(Ordering::Relaxed));
        assert_eq!(model.recovery(), before);
        assert_eq!(files.saved.as_ref(), SOURCE);
        assert!(!model.document().can_undo());
        let archive = files.external.as_ref().unwrap().archive().unwrap();
        let recovery: Recovery =
            serde_json::from_str(&fs::read_to_string(archive).unwrap()).unwrap();
        assert_eq!(recovery.draft, before);
    }
}

#[test]
fn late_recovery_failure_reports_adoption_after_real_replacement_or_deletion() {
    for keep in [true, false] {
        let (_directory, mut files, mut model) = fixture();
        model.set_draft("Archived pending prose.".into());
        files.checkpoint(&model).unwrap();
        let before = model.recovery();
        let disk = SOURCE.replace("Original.", "External.");
        fs::write(&files.current, &disk).unwrap();
        files.compare_external(&model).unwrap();
        let replacement = (!keep).then(|| {
            model
                .source_replacement_request(recite_writer_model::SourceValidation::Freeform)
                .prepare(
                    disk.clone().into(),
                    &recite_compiler::authoring::CancellationToken::new(),
                )
                .unwrap()
        });
        let receipt = files
            .publish_external_with_wait(
                &mut model,
                replacement,
                &recite_compiler::authoring::CancellationToken::new(),
                |fence| {
                    fence.wait()?;
                    Err(FileError::Io(std::io::Error::other("late directory sync")))
                },
            )
            .unwrap();
        assert!(receipt.recovery_error.is_some());
        assert!(files.external.is_none());
        assert_eq!(files.saved.as_ref(), disk);
        let archived: Recovery =
            serde_json::from_str(&fs::read_to_string(receipt.archive).unwrap()).unwrap();
        assert_eq!(archived.draft, before);
        if keep {
            assert_eq!(model.recovery(), before);
            assert!(files.has_recovery());
        } else {
            assert_eq!(model.document().source(), disk);
            assert!(!files.has_recovery());
            model.undo().unwrap();
            assert_eq!(model.document().source(), SOURCE);
        }
        assert_eq!(fs::read_to_string(&files.current).unwrap(), disk);
    }
}
