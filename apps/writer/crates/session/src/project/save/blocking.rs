//! Blocking adapter for callers outside a native event loop; uses the same staged publication.
use super::*;
impl ProjectFiles {
    pub fn save_current(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::Current)
    }
    pub fn save_all(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::All)
    }
    pub fn save_document(
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
}
