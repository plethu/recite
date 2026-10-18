use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{FileError, ProjectFiles, replace_checked, replace_checked_with_sync};

#[test]
fn retry_after_visible_replacement_confirms_sync_without_another_backup()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("scene.recite");
    fs::write(&path, "before")?;
    let error = replace_checked_with_sync(&path, "before", "after", |_| {
        Err(io::Error::other("directory sync failed"))
    });
    assert!(matches!(error, Err(FileError::Io(_))));
    assert_eq!(fs::read_to_string(&path)?, "after");
    let files_before_retry = fs::read_dir(root.path())?.count();
    replace_checked(&path, "before", "after")?;
    assert_eq!(fs::read_dir(root.path())?.count(), files_before_retry);
    fs::write(&path, "someone else's edit")?;
    assert!(matches!(
        replace_checked(&path, "before", "after"),
        Err(FileError::Conflict)
    ));
    assert_eq!(fs::read_to_string(path)?, "someone else's edit");
    Ok(())
}

#[cfg(unix)]
#[test]
fn desired_content_does_not_bypass_symlink_rejection() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let target = root.path().join("target.recite");
    let link = root.path().join("link.recite");
    fs::write(&target, "after")?;
    std::os::unix::fs::symlink(&target, &link)?;
    assert!(matches!(
        replace_checked(&link, "before", "after"),
        Err(FileError::FileKind)
    ));
    Ok(())
}

#[test]
fn active_save_shares_accepted_source_with_saved_inputs_and_unchanged_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = super::super::tests::project()?;
    let mut model = files.workbench()?;
    model.select(recite_writer_model::View::Source)?;
    let changed = model
        .document()
        .source()
        .replace("Would you tell me", "A changed courier asks");
    model.set_draft(changed);
    files.save_current(&mut model)?;
    let accepted = model.document().source_snapshot();
    assert!(std::sync::Arc::ptr_eq(&accepted, &files.saved));
    let saved = files
        .context
        .documents
        .iter()
        .find(|input| input.key() == model.document().key())
        .ok_or("saved input")?;
    assert_eq!(saved.text().as_ptr(), accepted.as_ptr());
    let search = files.search.clone();
    files.save_current(&mut model)?;
    assert!(std::sync::Arc::ptr_eq(&accepted, &files.saved));
    assert!(std::sync::Arc::ptr_eq(&search, &files.search));
    Ok(())
}

impl ProjectFiles {
    pub(crate) fn save(&mut self, source: &str) -> Result<(), FileError> {
        let source = if source == self.saved.as_ref() {
            self.saved.clone()
        } else {
            source.into()
        };
        self.save_shared(source)
    }
}

use criterion::measurement::{Measurement, WallTime};
use recite_compiler::authoring::CancellationToken;
use recite_writer_model::View;
use serde_json::{Value, json};

fn copy_project(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_project(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
fn measure_save_phase<T>(rows: &mut Vec<Value>, name: &str, operation: impl FnOnce() -> T) -> T {
    let start = WallTime.start();
    let value = operation();
    rows.push(json!({"operation":name,"ms":WallTime.end(start).as_secs_f64()*1000.}));
    value
}
#[test]
#[ignore = "explicit native-save phase investigation"]
fn source_apply_and_save_phases() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from(std::env::var_os("RECITE_UX_PROJECT").ok_or("RECITE_UX_PROJECT")?);
    let mut samples = Vec::new();
    for round in 0..3 {
        for phases in [true, false] {
            let dir = tempfile::tempdir()?;
            copy_project(&input, dir.path())?;
            let mut files = ProjectFiles::open_with_control(dir.path(), &CancellationToken::new())?;
            let mut model =
                files.workbench_with_control(Some(View::Source), &CancellationToken::new())?;
            let source = model.document().source().to_owned();
            let term = if source.contains("zenith") {
                "zenith"
            } else {
                "courier"
            };
            assert!(source.contains(term));
            let changed = source.replacen(term, "zeppelin", 1);
            model.set_draft(changed.clone());
            let mut rows = Vec::new();
            if phases {
                measure_save_phase(&mut rows, "apply", || model.apply())?;
                measure_save_phase(&mut rows, "save_disk_and_index", || {
                    files.save_shared(model.document().source_snapshot())
                })?;
                measure_save_phase(&mut rows, "checkpoint", || files.checkpoint(&model))?;
                measure_save_phase(&mut rows, "refresh_after_save", || {
                    files.refresh_after_save(&mut model, Ok(()))
                })?;
            } else {
                measure_save_phase(&mut rows, "save_current", || files.save_current(&mut model))?;
            }
            assert_eq!(fs::read_to_string(&files.current)?, changed);
            assert_eq!(model.document().source(), changed);
            assert!(!files.dirty(&changed));
            let expected = recite_writer_model::SearchIndex::build(&files.context.documents);
            assert_eq!(
                files.search.search("zeppelin", 100),
                expected.search("zeppelin", 100)
            );
            samples.push(
                json!({"round":round,"phases":phases,"source_bytes":source.len(),"samples":rows}),
            );
        }
    }
    println!("{}", json!({"corpus":input.file_name(),"samples":samples}));
    Ok(())
}

impl ProjectFiles {
    pub(crate) fn save_current(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::Current)
    }
    pub(crate) fn save_all(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::All)
    }
    pub(crate) fn save_document(
        &mut self,
        current: &mut recite_writer_model::Workbench,
        path: &Path,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::Document(path.to_owned()))
    }
    fn save_synchronously(
        &mut self,
        current: &mut recite_writer_model::Workbench,
        scope: super::ordered::Scope,
    ) -> Result<(), FileError> {
        let steps = self.save_plan(current, scope)?;
        let result = (|| {
            for step in steps {
                match step {
                    super::ordered::Step::Manifest => {
                        if self.manifest.dirty() {
                            self.manifest.save()?;
                        }
                    }
                    super::ordered::Step::Declarations { force } => {
                        if let Some(session) =
                            self.declarations.as_mut().filter(|s| force || s.dirty())
                        {
                            session.save_and_generate()?;
                            self.refresh(current)?;
                        }
                    }
                    step => {
                        let mut write = self.prepare_write(step)?;
                        write.execute()?;
                        let path = write.path.clone();
                        let result = self.publish_write(current, &mut write)?.wait();
                        let park = self.finish_saved_tab(&path);
                        result.and(park)?;
                    }
                }
            }
            Ok(())
        })();
        self.finish_save(current, result)
    }
    fn save_shared(&mut self, source: std::sync::Arc<str>) -> Result<(), FileError> {
        replace_checked(&self.current, &self.saved, &source)?;
        self.saved = source;
        self.update_saved_context()
    }
}
