use recite_core::{DiagnosticRecord, SourceSpan};

use super::{CatalogError, UiCatalog};

/// A diagnostic primary and its optional, independently rendered supporting
/// text for a host UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedDiagnostic {
    pub primary_text: String,
    pub related: Vec<RenderedRelatedDiagnostic>,
    pub help: Option<String>,
}

/// A rendered related diagnostic retains the source span supplied by the
/// producer so editors can attach the text to the same source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedRelatedDiagnostic {
    pub span: SourceSpan,
    pub text: String,
}

impl UiCatalog {
    /// Render a structured diagnostic through this catalog's deterministic
    /// locale fallback chain.
    ///
    /// The primary presentation is authoritative when it resolves to usable
    /// text. Its compatibility message is the temporary migration fallback.
    /// Related and help presentations are intentionally fail-closed: an
    /// unrenderable item does not become an untranslated compatibility string
    /// or prevent the rest of the record from being rendered.
    pub fn render_diagnostic(
        &self,
        record: &DiagnosticRecord,
    ) -> Result<RenderedDiagnostic, CatalogError> {
        let primary_text = self.render_primary(record)?;
        let related = record
            .related
            .iter()
            .filter_map(|item| {
                self.format_presentation(&item.presentation)
                    .ok()
                    .filter(|text| is_usable_text(text))
                    .map(|text| RenderedRelatedDiagnostic {
                        span: item.span.clone(),
                        text,
                    })
            })
            .collect();
        let help = record.help.as_ref().and_then(|presentation| {
            self.format_presentation(presentation)
                .ok()
                .filter(|text| is_usable_text(text))
        });

        Ok(RenderedDiagnostic {
            primary_text,
            related,
            help,
        })
    }

    fn render_primary(&self, record: &DiagnosticRecord) -> Result<String, CatalogError> {
        let presentation_result = self.format_presentation(&record.presentation);
        if let Ok(text) = &presentation_result
            && is_usable_text(text)
        {
            return Ok(text.clone());
        }

        if let Some(message) = record.compatibility_message()
            && is_usable_text(message)
        {
            return Ok(message.to_owned());
        }

        let details = match presentation_result {
            Ok(_) => "structured presentation produced no usable text".to_owned(),
            Err(error) => error.to_string(),
        };
        Err(CatalogError::Resolution {
            id: record.presentation.id().to_string(),
            details: format!("diagnostic primary presentation unavailable: {details}"),
        })
    }
}

fn is_usable_text(text: &str) -> bool {
    !text.trim().is_empty()
}
