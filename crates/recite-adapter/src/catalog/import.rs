use recite_core::po::PoDocument;
use recite_runtime::localisation::TextDomain;

use super::{AdapterError, AdapterErrorKind, AdapterResult, ReciteDialogueCatalog, valid_locale};

impl ReciteDialogueCatalog {
    /// Atomically merges a writer-owned gettext PO document into the catalogue.
    /// Fuzzy and obsolete entries are ignored, as in the CLI authoring preview.
    pub fn import_po(
        &mut self,
        locale: &str,
        source_name: &str,
        po_text: &str,
    ) -> AdapterResult<()> {
        let locale = valid_locale(locale)?;
        let document = PoDocument::parse_with_path(source_name, po_text).map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::Localisation, error.to_string())
        })?;
        // Stage only incoming changes. Existing rows remain untouched until
        // every row has validated, including conflicts in source order.
        let mut staged = Self::new();
        if let Some(header) = self.plural_forms.get(locale.as_str()) {
            staged
                .plural_forms
                .insert(locale.as_str().to_owned(), header.clone());
        }
        if let Some(header) = document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
        {
            if let Some(existing) = staged.plural_forms.get(locale.as_str())
                && existing != header.value()
            {
                return Err(AdapterError::with_detail(
                    AdapterErrorKind::Localisation,
                    format!("conflicting plural rule for `{locale}`"),
                ));
            }
            staged.set_plural_forms(locale.as_str(), header.value())?;
        }
        for entry in document.entries() {
            if entry.is_header()
                || entry.is_obsolete()
                || entry.flags().iter().any(|flag| flag == "fuzzy")
            {
                continue;
            }
            let context = entry.context().ok_or_else(|| {
                AdapterError::with_detail(
                    AdapterErrorKind::Localisation,
                    format!("{source_name}:{}: PO entry requires msgctxt", entry.line()),
                )
            })?;
            let (context, variant) = context
                .split_once('&')
                .map_or((context, None), |(base, variant)| (base, Some(variant)));
            let (domain, id) = if let Some(id) = context.strip_prefix("availability_reason:") {
                (TextDomain::AvailabilityReason, id)
            } else if let Some(id) = context.strip_prefix("presentation_label:") {
                (TextDomain::PresentationLabel, id)
            } else {
                (TextDomain::Line, context)
            };
            let (key, value) = if let Some(plural) = entry.plural_source_text() {
                if domain != TextDomain::Line {
                    return Err(AdapterError::with_detail(
                        AdapterErrorKind::Localisation,
                        format!(
                            "{source_name}:{}: plural entry must use a line context",
                            entry.line()
                        ),
                    ));
                }
                staged.prepare_plural(
                    locale.as_str(),
                    id,
                    entry.source_text(),
                    plural,
                    entry
                        .plural_translations()
                        .iter()
                        .map(|translation| translation.text().to_owned())
                        .collect(),
                    variant,
                )?
            } else {
                let text = entry.translation().unwrap_or_default();
                staged.prepare_singular(
                    locale.as_str(),
                    domain,
                    id,
                    entry.source_text(),
                    text,
                    variant,
                )?
            };
            self.check_value(&key, &value)?;
            staged.insert_value(key, value)?;
        }
        if self.translations.is_empty() {
            self.translations = staged.translations;
        } else {
            self.translations.extend(staged.translations);
        }
        self.plural_forms.extend(staged.plural_forms);
        Ok(())
    }
}
