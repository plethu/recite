use recite_writer_model::{EditError, WorkbenchError};
use recite_writer_session::FileError;

pub fn edit(error: &EditError) -> String {
    match error {
        EditError::SourceRequired(reason) => format!("Open this source in your editor: {reason}."),
        EditError::NotProse => "This draft would change Recite syntax or whitespace. Open the source in your editor; your draft remains recoverable.".into(),
        _ => error.to_string(),
    }
}

pub fn file(error: &FileError) -> String {
    match error {
        FileError::Workbench(error) => workbench(error),
        FileError::Saving { path, error } => {
            format!("Could not save {}: {}", path.display(), file(error))
        }
        _ => error.to_string(),
    }
}

pub fn workbench(error: &WorkbenchError) -> String {
    match error {
        WorkbenchError::Edit(error) => edit(error),
        _ => error.to_string(),
    }
}
