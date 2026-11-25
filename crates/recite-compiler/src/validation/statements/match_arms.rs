//! A `:match` must not have an arm traversal can never select.

use std::collections::BTreeMap;

use recite_core::SourceSpan;
use recite_core::ast::{MatchBranch, MatchPattern};

use super::super::participation::ValidationCompleteness;
use super::super::state::Validator;
use crate::diagnostics;

impl Validator<'_> {
    /// Traversal selects the first arm for a variant, or else the wildcard,
    /// so a repeated variant and any arm after `:case _` never run.
    pub(super) fn validate_match_arm_patterns(&mut self, branch: &MatchBranch) {
        if self.participation.ast_structure() != ValidationCompleteness::Complete {
            return;
        }
        let mut variants: BTreeMap<&str, SourceSpan> = BTreeMap::new();
        let mut wildcard: Option<SourceSpan> = None;
        for arm in &branch.arms {
            let pattern = match &arm.pattern {
                MatchPattern::Variant(variant) => variant.as_str(),
                MatchPattern::Wildcard => "_",
            };
            if let Some(wildcard_span) = &wildcard {
                self.diagnostics
                    .push(diagnostics::match_case_after_wildcard(
                        pattern,
                        arm.span.clone(),
                        wildcard_span.clone(),
                    ));
                continue;
            }
            match &arm.pattern {
                MatchPattern::Wildcard => wildcard = Some(arm.span.clone()),
                MatchPattern::Variant(variant) => {
                    if let Some(first) = variants.get(variant.as_str()) {
                        self.diagnostics.push(diagnostics::duplicate_match_case(
                            variant,
                            arm.span.clone(),
                            first.clone(),
                        ));
                    } else {
                        variants.insert(variant.as_str(), arm.span.clone());
                    }
                }
            }
        }
    }
}
