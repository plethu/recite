//! Reply rule edits preserve source outside the edited expressions and effect slots.
mod effect;
mod expression;
pub use effect::RuleEffect;
pub use expression::RuleExpression;
mod projection;
mod values;
use crate::{Document, EditError, Workbench, WorkbenchError, projection::offset};
use recite_core::{DocumentKey, SourceSpan};
use std::ops::Range;
pub use values::RuleArgument;

#[derive(Clone, Debug, PartialEq)]
pub struct ReplyRules {
    pub passage: String,
    /// The same-document destination whose direct effects are being edited.
    /// Other replies and diverts may also reach this block.
    pub destination: Option<String>,
    pub text: String,
    pub condition: Option<RuleExpression>,
    pub effects: Vec<RuleEffect>,
    pub effect_order_editable: bool,
    pub has_other_statements: bool,
    pub available_conditions: Vec<RuleExpression>,
    pub available_effects: Vec<RuleEffect>,
    key: DocumentKey,
    revision: i64,
    source: String,
    original_condition: Option<RuleExpression>,
    condition_range: Option<Range<usize>>,
    header_end: usize,
    requirement_range: Option<Range<usize>>,
    effect_insertion: Option<(usize, String)>,
    effect_ranges: Vec<Range<usize>>,
}
impl ReplyRules {
    pub fn belongs_to(&self, document: &Document) -> bool {
        document.key() == &self.key && document.revision() == self.revision
    }
    pub fn changed(&self) -> bool {
        self.source().is_ok_and(|source| source != self.source)
            || self.condition != self.original_condition
            || self
                .effects
                .iter()
                .any(|effect| effect.arguments.iter().any(RuleArgument::changed))
    }
    pub fn source(&self) -> Result<String, EditError> {
        let mut patches = Vec::new();
        if self.condition != self.original_condition {
            match (&self.condition, &self.condition_range) {
                (Some(condition), Some(range)) => {
                    patches.push((range.clone(), condition.source()?))
                }
                (Some(condition), None) => patches.push((
                    self.header_end..self.header_end,
                    format!(" requires=({})", condition.source()?),
                )),
                (None, Some(_)) => patches.push((
                    self.requirement_range.clone().ok_or(EditError::Position)?,
                    String::new(),
                )),
                (None, None) => {}
            }
        }
        if !self.effect_order_editable
            && (self.effects.len() != self.effect_ranges.len()
                || self
                    .effects
                    .iter()
                    .zip(&self.effect_ranges)
                    .any(|(effect, range)| {
                        effect.original.as_ref().is_none_or(|original| {
                            Some(original.span.start) != self.span_start(range).ok()
                        })
                    }))
        {
            return Err(EditError::SourceRequired(
                "effects cannot move across other statements",
            ));
        }
        for (index, range) in self.effect_ranges.iter().enumerate() {
            patches.push((
                range.clone(),
                self.effects
                    .get(index)
                    .map(RuleEffect::source)
                    .transpose()?
                    .unwrap_or_default(),
            ));
        }
        if self.effects.len() > self.effect_ranges.len() {
            let (at, indent) = self
                .effect_insertion
                .as_ref()
                .ok_or(EditError::SourceRequired(
                    "add effects in Source for this branch",
                ))?;
            let newline = if self.source.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let mut added = String::new();
            for effect in &self.effects[self.effect_ranges.len()..] {
                added.push_str(&format!("{indent}{}{newline}", effect.source()?));
            }
            patches.push((*at..*at, added));
        }
        replace(&self.source, patches)
    }
    fn span_start(&self, range: &Range<usize>) -> Result<recite_core::SourcePosition, EditError> {
        let prefix = self.source.get(..range.start).ok_or(EditError::Position)?;
        recite_core::SourcePosition::new(
            u32::try_from(prefix.bytes().filter(|b| *b == b'\n').count() + 1)
                .map_err(|_| EditError::Position)?,
            u32::try_from(
                prefix
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .count()
                    + 1,
            )
            .map_err(|_| EditError::Position)?,
        )
        .map_err(EditError::from)
    }
    /// Cheap validation for editing feedback; Apply also runs project diagnostics.
    pub fn validate_draft(&self, document: &Document) -> Result<String, EditError> {
        if document.key() != &self.key {
            return Err(EditError::Stale);
        }
        document.check_revision(self.revision)?;
        if let Some(condition) = &self.condition {
            condition.validate()?;
        }
        for effect in &self.effects {
            for argument in &effect.arguments {
                argument.validate()?;
            }
        }
        self.source()
    }
    pub fn validate(&self, document: &Document) -> Result<String, EditError> {
        let source = self.validate_draft(document)?;
        let candidate =
            Document::in_project(self.key.clone(), source.clone(), document.project_context())?;
        if !recite_parser::parse(self.key.as_str(), &source)
            .lower_source_file()
            .diagnostics
            .is_empty()
        {
            return Err(EditError::SourceRequired(
                "repair the rule syntax before applying",
            ));
        }
        // Existing project diagnostics may remain, but rule edits must not introduce new errors.
        let mut remaining = document.diagnostics();
        for diagnostic in candidate.diagnostics() {
            let Some(index) = remaining
                .iter()
                .position(|old| old.code == diagnostic.code && old.message == diagnostic.message)
            else {
                return Err(EditError::InvalidRules(diagnostic.message));
            };
            remaining.swap_remove(index);
        }
        Ok(source)
    }
}
impl Workbench {
    pub fn apply_reply_rules(&mut self, rules: &ReplyRules) -> Result<(), WorkbenchError> {
        let source = rules.validate(self.document())?;
        if self.draft() != source {
            return Err(EditError::Stale.into());
        }
        self.apply()
    }
}
fn replace(source: &str, mut edits: Vec<(Range<usize>, String)>) -> Result<String, EditError> {
    edits.sort_by_key(|(range, _)| range.start);
    if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
        return Err(EditError::Position);
    }
    let mut result = source.to_owned();
    for (range, text) in edits.into_iter().rev() {
        if result.get(range.clone()).is_none() {
            return Err(EditError::Position);
        }
        result.replace_range(range, &text);
    }
    Ok(result)
}

fn span_range(source: &str, span: &SourceSpan) -> Result<Range<usize>, EditError> {
    let start = offset(source, span.start)?;
    let last = offset(source, span.end.ok_or(EditError::Position)?)?;
    let end = last
        + source
            .get(last..)
            .and_then(|s| s.chars().next())
            .ok_or(EditError::Position)?
            .len_utf8();
    Ok(start..end)
}
