//! Standalone TOML stays in the user's editor; Writer owns validation and publication.
use super::*;
mod actions;
mod preview;
mod repair;
use recite_writer_session::declarations::{DeclarationAction, SourceStatus};
use std::path::{Path, PathBuf};
#[derive(Default)]
enum Validation {
    #[default]
    Unchecked,
    Valid(SourceStatus),
    Invalid(String),
}

#[derive(Default)]
pub(in crate::workspaces) struct Panel {
    path: String,
    import: String,
    preview: Vec<String>,
    page: usize,
    artifact: Option<PathBuf>,
    archive: Option<PathBuf>,
    validation: Validation,
    repair_open: bool,
    preview_open: bool,
}
impl Panel {
    pub fn invalidate_output(&mut self) {
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
    pub fn retire_export(&mut self) {
        self.artifact = None;
    }
    pub fn toggle_preview(&mut self) {
        self.preview_open = !self.preview_open;
    }
    pub fn binding_path(&self) -> &str {
        &self.path
    }
    pub fn import_path(&self) -> &str {
        &self.import
    }
    pub fn exported(&self) -> Option<&Path> {
        self.artifact.as_deref()
    }
    pub fn record_artifacts(&mut self, archive: Option<PathBuf>, editable: Option<PathBuf>) {
        if let Some(path) = archive {
            self.archive = Some(path);
        }
        if let Some(path) = editable {
            self.import = path.to_string_lossy().into_owned();
            self.artifact = Some(path);
            self.repair_open = true;
        }
    }
    pub fn replace_preview(
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
    fn message(&self) -> &'static str {
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
pub(super) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        binding(writer),
        bound_source(writer),
        writer
            .declaration_session()
            .is_some_and(|session| session.busy() && !session.generation_active())
            .then(|| {
                text_button(
                    scale,
                    "Cancel declaration action",
                    Writer::cancel_declaration_action,
                )
            }),
        super::outcome::panel(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn binding(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if writer
        .declaration_session()
        .is_none_or(|session| session.source.is_some())
    {
        return None;
    }
    let scale = writer.ui_scale();
    let blocked = writer.declaration_action_blocked();
    Some(flex_col((
        label(scale, "Standalone declaration source (TOML)").text_size(18. * scale),
        paragraph(scale, "Bind your standalone TOML to edit it in your preferred editor. Writer validates explicit reloads and imports, then generates read-only JSON.", 640.),
        label(scale, "Declaration source path"),
        text_input(writer.workspaces.declarations.source.path.clone(), |writer: &mut Writer, path| writer.workspaces.declarations.source.path = path)
            .placeholder("Absolute path or path relative to this project").text_size(16. * scale)
            .disabled(blocked).prop(TextAreaLabel::new("Declaration source path")),
        text_button(scale, "Bind standalone source", Writer::bind_declaration_source)
            .disabled(blocked || writer.workspaces.declarations.source.path.trim().is_empty()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()))
}
fn bound_source(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let session = writer.declaration_session()?;
    let source = session.source.as_ref()?;
    let panel = &writer.workspaces.declarations.source;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(
                scale,
                if session.dirty() {
                    "Standalone TOML · draft or recovery needs saving"
                } else {
                    "Standalone TOML · source saved"
                },
            )
            .text_size(18. * scale),
            paragraph(scale, panel.status_message(), 640.),
            actions::primary(writer),
            paragraph(
                scale,
                format!("Saved source: {}", source.path.display()),
                640.,
            ),
            actions::recovery(writer),
            repair::panel(writer),
            preview::panel(writer),
            panel.archive.as_ref().map(|path| {
                paragraph(
                    scale,
                    format!("Previous draft preserved at {}", path.display()),
                    640.,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
}
