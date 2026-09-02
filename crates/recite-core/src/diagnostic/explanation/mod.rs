use crate::{
    DiagnosticCategory, DiagnosticCode, DiagnosticExplanationPresentation, DiagnosticPresentation,
    DiagnosticPresentationId,
};

mod freshness;
mod identifiers;
mod parse;
mod project;
mod schema;
mod validation;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub struct DiagnosticExplanation {
    pub code: DiagnosticCode,
    pub category: DiagnosticCategory,
    pub meaning: &'static str,
    pub common_causes: &'static [&'static str],
    pub remediation: &'static [&'static str],
}

impl DiagnosticExplanation {
    pub(crate) const fn new(
        code: &'static str,
        category: DiagnosticCategory,
        meaning: &'static str,
        common_causes: &'static [&'static str],
        remediation: &'static [&'static str],
    ) -> Self {
        Self {
            code: DiagnosticCode::new_static(code),
            category,
            meaning,
            common_causes,
            remediation,
        }
    }

    /// Return the locale-neutral presentation references for this explanation.
    ///
    /// The existing prose fields remain available as the compatibility view
    /// used by the current CLI. New clients should resolve these stable IDs
    /// through the shared UI catalogue instead of treating prose as identity.
    #[must_use]
    #[allow(
        clippy::expect_used,
        reason = "presentation IDs are derived through the validated diagnostic ID grammar"
    )]
    pub fn presentation(&self) -> DiagnosticExplanationPresentation {
        let meaning =
            DiagnosticPresentation::new(detail_presentation_id(&self.code, "meaning", None));
        let common_causes = self
            .common_causes
            .iter()
            .enumerate()
            .map(|(index, _cause)| {
                DiagnosticPresentation::new(detail_presentation_id(
                    &self.code,
                    "cause",
                    Some(index + 1),
                ))
            })
            .collect::<Vec<_>>();
        let remediation = self
            .remediation
            .iter()
            .enumerate()
            .map(|(index, _step)| {
                DiagnosticPresentation::new(detail_presentation_id(
                    &self.code,
                    "remediation",
                    Some(index + 1),
                ))
            })
            .collect::<Vec<_>>();
        DiagnosticExplanationPresentation::new(meaning)
            .with_common_causes(common_causes)
            .with_remediation(remediation)
    }

    /// Stable Fluent-compatible default presentation ID derived from the
    /// diagnostic code. A code may have additional variant presentations in
    /// the producer contract registry.
    #[must_use]
    pub fn default_code_presentation_id(&self) -> DiagnosticPresentationId {
        default_presentation_id_for_code(&self.code)
    }
}

/// Return a deterministic default Fluent-compatible presentation ID for any
/// valid diagnostic code. Producer contracts may register additional IDs for
/// variants that cannot share this default argument signature.
///
/// The established `RECITE_<FAMILY><NNN>` shape retains its readable
/// kebab-case projection. Other valid namespaced codes use an injective hex
/// projection so future underscores or non-numeric suffixes cannot silently
/// collide after sanitisation.
#[must_use]
pub fn default_presentation_id_for_code(code: &DiagnosticCode) -> DiagnosticPresentationId {
    let value = code
        .as_str()
        .strip_prefix("RECITE_")
        .unwrap_or(code.as_str());
    if let Some(number_start) = value.find(|character: char| character.is_ascii_digit()) {
        let namespace = &value[..number_start];
        let digits = &value[number_start..];
        if !namespace.is_empty()
            && namespace
                .chars()
                .all(|character| character.is_ascii_uppercase())
            && digits.chars().all(|character| character.is_ascii_digit())
        {
            return DiagnosticPresentationId::from_validated(format!(
                "diagnostic-{}-{}",
                namespace.to_ascii_lowercase(),
                digits
            ));
        }
    }

    let encoded = code
        .as_str()
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    DiagnosticPresentationId::from_validated(format!("diagnostic-code-{encoded}"))
}

#[allow(
    clippy::expect_used,
    reason = "detail presentation IDs are assembled from validated code and ordinal parts"
)]
fn detail_presentation_id(
    code: &DiagnosticCode,
    kind: &str,
    ordinal: Option<usize>,
) -> DiagnosticPresentationId {
    let default_id = default_presentation_id_for_code(code);
    let value = match ordinal {
        Some(ordinal) => format!("{}-{kind}-{ordinal:03}", default_id.as_str()),
        None => format!("{}-{kind}", default_id.as_str()),
    };
    DiagnosticPresentationId::new(value).expect("generated detail presentation ID is valid")
}

const GROUPS: &[&[DiagnosticExplanation]] = &[
    freshness::EXPLANATIONS,
    identifiers::EXPLANATIONS,
    parse::EXPLANATIONS,
    project::EXPLANATIONS,
    schema::EXPLANATIONS,
    validation::EXPLANATIONS,
];

pub fn known_diagnostic_explanations() -> impl Iterator<Item = &'static DiagnosticExplanation> {
    GROUPS.iter().flat_map(|group| group.iter())
}

#[must_use]
pub fn explain_diagnostic_code(code: &DiagnosticCode) -> Option<&'static DiagnosticExplanation> {
    known_diagnostic_explanations().find(|explanation| explanation.code.as_str() == code.as_str())
}

#[must_use]
pub fn suggest_diagnostic_code(input: &str) -> Option<&'static DiagnosticExplanation> {
    let normalized = input.to_ascii_uppercase();
    let candidate = if normalized.is_empty() {
        input
    } else {
        normalized.as_str()
    };
    let mut best: Option<(&DiagnosticExplanation, usize)> = None;

    for explanation in known_diagnostic_explanations() {
        let distance = edit_distance(candidate, explanation.code.as_str());
        if distance > 2 {
            continue;
        }

        match best {
            Some((_, best_distance)) if best_distance <= distance => {}
            _ => best = Some((explanation, distance)),
        }
    }

    best.map(|(explanation, _)| explanation)
}

fn edit_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }

    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (left_index, left_byte) in left.bytes().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_byte) in right.bytes().enumerate() {
            let substitution = usize::from(left_byte != right_byte);
            let insert = current[right_index] + 1;
            let delete = previous[right_index + 1] + 1;
            let replace = previous[right_index] + substitution;
            current[right_index + 1] = insert.min(delete).min(replace);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}
