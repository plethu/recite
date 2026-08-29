//! Bounded gettext plural expression parsing and evaluation.
//!
//! Catalogue loading and runtime providers share these modules so arm
//! selection stays deterministic and bounded.

mod ast;
mod evaluate;
mod parse;
mod validation;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PluralRuleError {
    #[error("invalid Plural-Forms header")]
    InvalidHeader,
    #[error("plural count must be non-negative")]
    NegativeCount,
    #[error("plural expression arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("plural expression divided by zero")]
    DivisionByZero,
    #[error("plural expression selected arm {arm}, but nplurals is {nplurals}")]
    ArmOutOfRange { arm: i64, nplurals: usize },
}

pub(super) fn validate_header(value: &str) -> Result<(usize, String), PluralRuleError> {
    let (nplurals, expression, parsed) =
        parse_header_parts(value).ok_or(PluralRuleError::InvalidHeader)?;
    parsed.validate(nplurals)?;
    Ok((nplurals, expression))
}

fn parse_header_parts(value: &str) -> Option<(usize, String, ast::Expr)> {
    let mut nplurals = None;
    let mut plural = None;
    let mut parsed_plural = None;
    let mut seen_nplurals = false;
    let mut seen_plural = false;
    for part in value.split(';') {
        let Some((key, value)) = part.trim().split_once('=') else {
            continue;
        };
        match key.trim() {
            "nplurals" => {
                if seen_nplurals {
                    return None;
                }
                seen_nplurals = true;
                nplurals = Some(value.trim().parse().ok().filter(|value| *value > 0)?);
            }
            "plural" => {
                if seen_plural {
                    return None;
                }
                seen_plural = true;
                let expression = value.trim();
                let parsed = parse::parse_expression(expression).ok()?;
                plural = Some(expression.to_owned());
                parsed_plural = Some(parsed);
            }
            _ => {}
        }
    }
    Some((nplurals?, plural?, parsed_plural?))
}

pub fn evaluate_plural_form(value: &str, count: i64) -> Result<usize, PluralRuleError> {
    if count < 0 {
        return Err(PluralRuleError::NegativeCount);
    }
    let (nplurals, _, expression) =
        parse_header_parts(value).ok_or(PluralRuleError::InvalidHeader)?;
    let arm = expression.evaluate(count)?;
    if arm < 0 || usize::try_from(arm).map_or(true, |arm| arm >= nplurals) {
        return Err(PluralRuleError::ArmOutOfRange { arm, nplurals });
    }
    Ok(arm as usize)
}

/// Validates a gettext `Plural-Forms` header for every non-negative `i64`
/// count represented by the bounded expression validator.
///
/// Runtime lookup still uses [`evaluate_plural_form`] for one count, while
/// catalogue owners should call this function when accepting a rule so an
/// invalid arm, overflow, or arithmetic fault cannot be deferred to a rare
/// runtime count.
pub fn validate_plural_rule(value: &str) -> Result<usize, PluralRuleError> {
    validate_header(value).map(|(nplurals, _)| nplurals)
}
