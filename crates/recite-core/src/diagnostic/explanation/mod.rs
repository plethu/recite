use super::{DiagnosticCategory, DiagnosticCode};

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
