//! Reconcile canonical buffers for every adopted receipt, including incomplete recovery.
use super::*;
pub(super) fn resolve(mut writer: Writer, decision: ExternalDecision) {
    writer.buffers.harvest();
    let result = {
        let mut model = writer.buffers.model.write();
        let mut files = writer.files.write();
        match (model.as_mut(), files.as_mut()) {
            (Ok(model), Some(files)) => files
                .resolve_external(model, decision)
                .map_err(|e| e.to_string()),
            _ => Err("Open a project first.".into()),
        }
    };
    match result {
        Ok(resolution) => {
            writer.buffers.sync(writer.dark);
            writer.pane.set(Pane::Map);
            let message = match decision {
                ExternalDecision::KeepWriter => {
                    "Kept Writer's document and draft. The next Save replaces the inspected disk version."
                }
                ExternalDecision::UseInspectedDisk => "Loaded the inspected disk version.",
            };
            let message = format!(
                "{message} Previous session retained at {}",
                resolution.archive.display()
            );
            if let Some(error) = resolution.recovery_error {
                writer.message.error(format!("{message} Recovery is incomplete: {error}. Retry Save or keep this window open."));
            } else {
                writer.message.info(message);
            }
        }
        Err(e) => writer.message.error(e),
    }
}
