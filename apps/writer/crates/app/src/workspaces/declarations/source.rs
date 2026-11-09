//! Standalone TOML stays in the user's editor; Writer owns validation and publication.
use recite_writer_session::declarations::SourceStatus;
use std::path::{Path, PathBuf};
#[derive(Default)]
pub enum Validation {
    #[default]
    Unchecked,
    Valid(SourceStatus),
    Invalid(String),
}

#[derive(Default)]
pub struct Panel {
    pub path: String,
    pub import: String,
    pub preview: Vec<String>,
    pub page: usize,
    pub artifact: Option<PathBuf>,
    pub archive: Option<PathBuf>,
    pub validation: Validation,
    pub repair_open: bool,
    pub preview_open: bool,
}
impl Panel {
    pub(crate) fn invalidate_output(&mut self) {
        if matches!(self.validation, Validation::Valid(_)) {
            self.validation = Validation::Unchecked;
        }
    }
    pub fn status_message(&self) -> &'static str {
        if matches!(self.validation, Validation::Invalid(_)) && self.artifact.is_some() {
            "Recovered draft exported. Open it in your editor, then import the repaired TOML."
        } else {
            self.validation.message()
        }
    }
    pub(crate) fn retire_export(&mut self) {
        self.artifact = None;
    }
    pub fn toggle_preview(&mut self) {
        self.preview_open = !self.preview_open;
    }
    pub(crate) fn binding_path(&self) -> &str {
        &self.path
    }
    pub fn import_path(&self) -> &str {
        &self.import
    }
    pub fn exported(&self) -> Option<&Path> {
        self.artifact.as_deref()
    }
    pub(crate) fn record_artifacts(&mut self, archive: Option<PathBuf>, editable: Option<PathBuf>) {
        if let Some(path) = archive {
            self.archive = Some(path);
        }
        if let Some(path) = editable {
            self.import = path.to_string_lossy().into_owned();
            self.artifact = Some(path);
            self.repair_open = true;
        }
    }
    pub(crate) fn replace_preview(
        &mut self,
        preview: Vec<String>,
        validation: &Result<SourceStatus, String>,
    ) {
        self.preview = preview;
        self.page = 0;
        self.validation = match validation {
            Ok(status) => Validation::Valid(*status),
            Err(error) => Validation::Invalid(
                super::super::text::pages(error)
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
            ),
        };
    }
}
impl Validation {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unchecked => {
                "Generated schema changed. Validate this draft to check whether it needs generation."
            }
            Self::Valid(SourceStatus::Current) => {
                "Source validation passed; generated schema is current."
            }
            Self::Valid(SourceStatus::NeedsGeneration) => {
                "Source validation passed. Save and generate to update the schema."
            }
            Self::Valid(SourceStatus::DamagedOutput) => {
                "Source validation passed; generated schema is damaged. Save and generate to repair it."
            }
            Self::Invalid(_) => {
                "Source needs repair. Export the recovered draft to edit it safely."
            }
        }
    }
}
