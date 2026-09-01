use std::collections::BTreeSet;

use recite_core::{SourceText, extract_placeholder_names};

use super::super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_interpolation(
        &mut self,
        source_text: &SourceText,
        bindings: &[recite_core::InterpolationBinding],
    ) {
        self.validate_interpolation_with_ignored(source_text, bindings, &[]);
    }

    fn validate_interpolation_with_ignored(
        &mut self,
        source_text: &SourceText,
        bindings: &[recite_core::InterpolationBinding],
        ignored_unused: &[&str],
    ) {
        let placeholders = match extract_placeholder_names(&source_text.text) {
            Ok(names) => names,
            Err(error) => {
                let error = match error.kind() {
                    recite_core::PlaceholderSyntaxKind::Unterminated => {
                        diagnostics::InterpolationError::Unterminated
                    }
                    recite_core::PlaceholderSyntaxKind::UnescapedClosingBrace => {
                        diagnostics::InterpolationError::UnescapedClosingBrace
                    }
                    recite_core::PlaceholderSyntaxKind::InvalidName(name) => {
                        diagnostics::InterpolationError::InvalidName(name.clone())
                    }
                    _ => unreachable!("placeholder syntax taxonomy is exhaustive"),
                };
                self.diagnostics.push(diagnostics::invalid_interpolation(
                    source_text.span.clone(),
                    error,
                ));
                return;
            }
        };
        let mut declared = BTreeSet::new();
        for binding in bindings {
            if !declared.insert(binding.name.as_str()) {
                self.diagnostics.push(diagnostics::invalid_interpolation(
                    source_text.span.clone(),
                    diagnostics::InterpolationError::Duplicate(binding.name.clone()),
                ));
            }
            if !placeholders.contains(&binding.name)
                && !ignored_unused.contains(&binding.name.as_str())
            {
                self.diagnostics.push(diagnostics::invalid_interpolation(
                    source_text.span.clone(),
                    diagnostics::InterpolationError::Unused(binding.name.clone()),
                ));
            }
        }
        for name in placeholders {
            if !declared.contains(name.as_str()) {
                self.diagnostics.push(diagnostics::invalid_interpolation(
                    source_text.span.clone(),
                    diagnostics::InterpolationError::Unbound(name),
                ));
            }
        }
    }

    pub(super) fn validate_plural_line(
        &mut self,
        source_file: &'a recite_core::SourceFile,
        line: &'a recite_core::Line,
        plural_source_text: &'a SourceText,
    ) {
        self.validate_source_text(
            source_file,
            plural_source_text,
            diagnostics::SourceSpanOwner::PluralSourceText,
            self.participation,
        );
        if line.source_text.text.contains('\n') || plural_source_text.text.contains('\n') {
            self.diagnostics.push(diagnostics::invalid_plural_line(
                line.span.clone(),
                diagnostics::PluralError::Newline,
            ));
        }
        let Some(count) = line
            .interpolation_bindings
            .iter()
            .find(|binding| binding.name == "count")
        else {
            self.diagnostics.push(diagnostics::invalid_plural_line(
                plural_source_text.span.clone(),
                diagnostics::PluralError::MissingCount,
            ));
            self.validate_interpolation(&line.source_text, &line.interpolation_bindings);
            self.validate_interpolation(plural_source_text, &line.interpolation_bindings);
            return;
        };
        if count.value_type != recite_core::InterpolationType::Integer {
            self.diagnostics.push(diagnostics::invalid_plural_line(
                line.span.clone(),
                diagnostics::PluralError::CountType,
            ));
        }
        let singular_placeholders =
            extract_placeholder_names(&line.source_text.text).unwrap_or_default();
        let plural_placeholders =
            extract_placeholder_names(&plural_source_text.text).unwrap_or_default();
        let singular_unused = line
            .interpolation_bindings
            .iter()
            .filter(|binding| {
                !singular_placeholders.contains(&binding.name)
                    && (binding.name == "count" || plural_placeholders.contains(&binding.name))
            })
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>();
        let plural_unused = line
            .interpolation_bindings
            .iter()
            .filter(|binding| {
                !plural_placeholders.contains(&binding.name)
                    && (binding.name == "count" || singular_placeholders.contains(&binding.name))
            })
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>();
        self.validate_interpolation_with_ignored(
            &line.source_text,
            &line.interpolation_bindings,
            &singular_unused,
        );
        self.validate_interpolation_with_ignored(
            plural_source_text,
            &line.interpolation_bindings,
            &plural_unused,
        );
    }
}
