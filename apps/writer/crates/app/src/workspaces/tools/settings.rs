use super::*;
use crate::{
    external_editor::Request as EditorRequest,
    workspaces::{ContextUpdate, text},
};
use recite_writer_session::project::settings::{
    SettingsAccepted, SettingsAction, SettingsCommit, SettingsOperation, SettingsReceipt,
};
#[cfg(any(test, feature = "test-support"))]
pub mod tests;

pub struct Prepared {
    receipt: SettingsReceipt,
    preview: Option<Vec<String>>,
}
impl Prepared {
    pub fn new(receipt: SettingsReceipt) -> Self {
        Self {
            preview: receipt.preview().map(text::pages),
            receipt,
        }
    }
}
impl Writer {
    pub fn project_settings_action(&mut self, action: SettingsAction) {
        if self.project_action_blocked() {
            return;
        }
        let result = self
            .session
            .project_mut()
            .ok_or("Open a project first.".to_owned())
            .and_then(|project| {
                project
                    .parts_mut()
                    .0
                    .prepare_settings(action)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(work) => self.submit_tool(Work::Settings(Box::new(work))),
            Err(error) => self.settings_feedback(error),
        }
    }
    pub fn import_project_settings(&mut self) {
        let path = self.workspaces.settings.import_path();
        if !path.is_empty() {
            self.project_settings_action(SettingsAction::ImportDraft(path.into()));
        }
    }
    pub fn apply_project_settings(&mut self) {
        if self.project_action_blocked() {
            return;
        }
        let result = self
            .session
            .project_mut()
            .ok_or("Open a project first.".to_owned())
            .and_then(|project| {
                let (files, current) = project.parts_mut();
                files
                    .prepare_settings_publication(current)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(work) => self.submit_tool(Work::SettingsPublish(Box::new(work))),
            Err(error) => self.settings_feedback(error),
        }
    }
    pub fn open_settings_draft(&mut self) {
        if self.external.busy() || self.project_action_blocked() {
            return;
        }
        let Some(project) = self.session.project() else {
            return;
        };
        let Some(path) = self.workspaces.settings.artifact().map(PathBuf::from) else {
            return;
        };
        self.launch_editor(Arc::new(EditorRequest::settings_draft(
            project,
            path,
            self.editor.clone(),
        )));
    }
    pub fn open_project_manifest(&mut self) {
        if self.external.busy() || self.project_action_blocked() {
            return;
        }
        let Some(project) = self.session.project() else {
            return;
        };
        let Some(session) = project.files().settings.as_ref() else {
            return;
        };
        self.launch_editor(Arc::new(EditorRequest::project_manifest(
            project,
            session.path().to_owned(),
            self.editor.clone(),
        )));
    }
    pub fn accept_settings_tool(&mut self, prepared: &mut Prepared) -> Result<(), String> {
        if let Some(path) = prepared.receipt.artifact() {
            self.workspaces.settings.record_artifact(path.to_owned());
        }
        let accepted = self
            .session
            .project_mut()
            .ok_or("Project settings were closed.".to_owned())?
            .parts_mut()
            .0
            .accept_settings(&mut prepared.receipt)
            .map_err(|error| match prepared.receipt.operation() {
                SettingsOperation::Import => {
                    format!("Settings draft import failed. Saved manifest unchanged.\n{error}")
                }
                _ => error.to_string(),
            });
        if self.declaration_session().is_none() {
            self.workspaces.declarations.replace(vec![]);
            self.workspaces.declarations.source.invalidate_output();
        }
        if accepted.is_err()
            && self
                .project_settings()
                .is_some_and(|session| session.requires_reload())
        {
            self.workspaces.context_refresh = Some(ContextUpdate::SettingsUnconfirmed);
        }
        let accepted = accepted?;
        if let Some(preview) = prepared.preview.take() {
            self.workspaces.settings.replace_preview(preview);
        }
        if accepted == SettingsAccepted::ContextChanged {
            self.workspaces.context_refresh = Some(ContextUpdate::SettingsReloaded);
            if self.declaration_session().is_none() {
                self.workspaces.declarations.replace(vec![]);
                self.workspaces.declarations.source.invalidate_output();
            }
            self.refresh_project_context();
            return Ok(());
        }
        let message = if accepted == SettingsAccepted::Reloaded {
            "Saved project settings reloaded. Export a draft to edit them."
        } else if self
            .project_settings()
            .is_some_and(|session| session.reviewed_source().is_some())
        {
            "Settings draft imported. Ready to Apply."
        } else {
            "Project settings prepared. The saved manifest stays unchanged until Apply."
        };
        self.workspaces.settings.clear_feedback();
        self.status = message.into();
        Ok(())
    }
    pub fn accept_settings_written(&mut self, receipt: &mut SettingsWritten) -> Result<(), String> {
        let project = self
            .session
            .project_mut()
            .ok_or("The project was closed.".to_owned())?;
        let result = project
            .parts_mut()
            .0
            .accept_settings_publication(receipt)
            .map_err(|error| error.to_string())?;
        if project.files().declarations.is_none() {
            self.workspaces.declarations.replace(vec![]);
            self.workspaces.declarations.source.invalidate_output();
        }
        let update = match result {
            SettingsCommit::Published => {
                self.workspaces.settings.clear_feedback();
                ContextUpdate::SettingsSaved
            }
            SettingsCommit::PublishedWithWarning(error) => {
                self.settings_feedback(format!(
                    "Project settings saved with a storage warning:\n{error}"
                ));
                ContextUpdate::SettingsSaved
            }
            SettingsCommit::Unconfirmed(error) => {
                self.settings_feedback(format!("Saved project settings need confirmation. Inspect the saved manifest and Reload saved settings:\n{error}"));
                ContextUpdate::SettingsUnconfirmed
            }
        };
        self.workspaces.context_refresh = Some(update);
        self.refresh_project_context();
        Ok(())
    }
}
