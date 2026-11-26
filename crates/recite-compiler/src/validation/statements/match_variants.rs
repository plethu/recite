//! With a schema, a `:match` must name only variants its scrutinee's enum
//! declares, and must cover every declared variant unless it has `:case _`.

use std::collections::BTreeSet;

use recite_core::ast::{MatchBranch, MatchPattern};
use recite_core::schema::{ConditionReturnType, SchemaTypeDefinition};

use super::super::participation::ValidationCompleteness;
use super::super::state::Validator;
use crate::diagnostics;

impl Validator<'_> {
    pub(super) fn validate_match_arm_variants(&mut self, branch: &MatchBranch) {
        if self.participation.ast_structure() != ValidationCompleteness::Complete
            || self.participation.condition_functions() != ValidationCompleteness::Complete
        {
            return;
        }
        let Some(schema) = self.schema else {
            return;
        };
        // An unknown scrutinee or a non-enum return is reported by the
        // scrutinee's own checks; there is no enum to compare arms with.
        let Some(ConditionReturnType::Enum(enum_name)) = schema
            .conditions
            .get(branch.scrutinee.function.as_str())
            .map(|definition| &definition.returns)
        else {
            return;
        };
        let Some(SchemaTypeDefinition::Enum(definition)) = schema.types.get(enum_name) else {
            return;
        };
        let mut covered = BTreeSet::new();
        let mut wildcard = false;
        for arm in &branch.arms {
            match &arm.pattern {
                MatchPattern::Wildcard => wildcard = true,
                MatchPattern::Variant(variant) if definition.values.contains(variant) => {
                    covered.insert(variant.as_str());
                }
                MatchPattern::Variant(variant) => {
                    let suggestion = super::super::suggest::closest(
                        variant,
                        definition.values.iter().map(String::as_str),
                    );
                    self.diagnostics.push(diagnostics::unknown_match_variant(
                        variant,
                        enum_name,
                        arm.span.clone(),
                        suggestion,
                    ));
                }
            }
        }
        if wildcard {
            return;
        }
        let missing = definition
            .values
            .iter()
            .map(String::as_str)
            .filter(|variant| !covered.contains(variant))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.diagnostics.push(diagnostics::non_exhaustive_match(
                enum_name,
                &missing,
                branch.span.clone(),
            ));
        }
    }
}
