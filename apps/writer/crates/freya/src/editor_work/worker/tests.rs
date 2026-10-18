use super::*;
use crate::project::{ProjectFiles, save::ordered::Step};
use recite_writer_model::View;
use std::sync::Arc;

#[test]
fn saved_search_owners_retire_on_worker_before_recovery_even_on_failure()
-> Result<(), Box<dyn std::error::Error>> {
    for case in ["saved", "recovery_failed", "disk_conflict"] {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("recite.project.toml"),
            "format_version = 1\n",
        )?;
        let path = dir.path().join("scene.recite");
        let source = ":: start default\n> greeting@11111111111111111111\n  Hello.\n-> END\n";
        std::fs::write(&path, source)?;
        let mut files = ProjectFiles::open(dir.path())?;
        let mut model = files.workbench_with_control(
            Some(View::Source),
            &recite_compiler::authoring::CancellationToken::new(),
        )?;
        model.set_draft(source.replace("Hello", "Updated"));
        model.apply()?;
        if case == "recovery_failed" {
            files.checkpoint(&model)?;
            let sidecar = path.with_file_name("scene.recite.recite-editor-recovery.json");
            std::fs::remove_file(&sidecar)?;
            std::fs::create_dir(&sidecar)?;
        }
        let index = files.search_index();
        let old = Arc::downgrade(&index);
        drop(index);
        let write = files.prepare_write(Step::Current(model.document().source_snapshot()))?;
        if case == "disk_conflict" {
            std::fs::write(&path, source.replace("Hello", "External"))?;
        }
        let worker = Worker::new()?;
        futures_lite::future::block_on(async {
            let Output::Written(mut write, result) = worker.execute(Work::Write(write)).await?
            else {
                return Err("unexpected write receipt".into());
            };
            if case == "disk_conflict" {
                assert!(result.is_err());
                assert!(matches!(
                    worker.execute(Work::RetireWrite(write)).await?,
                    Output::Retired
                ));
                assert_eq!(old.strong_count(), 1);
                assert_eq!(
                    files.search_index().search_results("updated", 10).total(),
                    0
                );
                return Ok(());
            }
            result?;
            let ticket = files.publish_write(&model, &mut write)?;
            // Only the receipt owns the displaced index, including after a failed cleanup.
            assert_eq!(old.strong_count(), 1);
            assert_eq!(
                files.search_index().search_results("updated", 10).total(),
                1
            );
            assert!(matches!(
                worker.execute(Work::RetireWrite(write)).await?,
                Output::Retired
            ));
            assert!(old.upgrade().is_none());
            let Output::Recovered(result) = worker.execute(Work::Recovery(ticket)).await? else {
                return Err("unexpected recovery receipt".into());
            };
            assert_eq!(result.is_err(), case == "recovery_failed");
            assert_eq!(std::fs::read_to_string(&path)?, model.document().source());
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    }
    Ok(())
}
impl super::Worker {
    pub(crate) fn for_test() -> Self {
        Self::new().unwrap()
    }
}
