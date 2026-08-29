//! Whole-domain validation for parsed gettext plural rules.
//!
//! The runtime evaluator handles one count at a time. This module instead
//! propagates conservative value ranges for every non-negative `i64` count so
//! catalogue loading can reject rules that can fault or select an invalid arm.
//! Guards refine the count domain along conditional and short-circuit paths;
//! expressions whose correlations cannot be proved by this interval model are
//! intentionally rejected rather than sampled or accepted optimistically.

use super::PluralRuleError;
use super::ast::{Binary, Expr, Unary};

mod range;
mod refine;

use range::{CountDomain, ValueRange};
use refine::{comparison_range, refine_domain};

impl Expr {
    pub(super) fn validate(&self, nplurals: usize) -> Result<(), PluralRuleError> {
        let range = self.value_range(CountDomain::ALL)?;
        if range.min < 0 {
            return Err(PluralRuleError::ArmOutOfRange {
                arm: range.min,
                nplurals,
            });
        }
        if i128::from(range.max) >= nplurals as i128 {
            return Err(PluralRuleError::ArmOutOfRange {
                arm: range.max,
                nplurals,
            });
        }
        Ok(())
    }

    fn value_range(&self, domain: CountDomain) -> Result<ValueRange, PluralRuleError> {
        match self {
            Self::Number(value) => Ok(ValueRange::exact(*value)),
            Self::N => Ok(ValueRange::from_domain(domain)),
            Self::Unary(Unary::Not, expression) => {
                expression.value_range(domain)?;
                Ok(ValueRange { min: 0, max: 1 })
            }
            Self::Unary(Unary::Plus, expression) => expression.value_range(domain),
            Self::Unary(Unary::Minus, expression) => {
                let range = expression.value_range(domain)?;
                ValueRange::from_i128(-i128::from(range.max), -i128::from(range.min))
            }
            Self::Conditional(condition, when_true, when_false) => {
                let condition_range = condition.value_range(domain)?;
                if condition_range.is_exact(0) {
                    return when_false.value_range(domain);
                }
                if condition_range.always_nonzero() {
                    return when_true.value_range(domain);
                }
                let true_range = refine_domain(condition, domain, true)
                    .map(|domain| when_true.value_range(domain))
                    .transpose()?;
                let false_range = refine_domain(condition, domain, false)
                    .map(|domain| when_false.value_range(domain))
                    .transpose()?;
                match (true_range, false_range) {
                    (Some(true_range), Some(false_range)) => Ok(true_range.union(false_range)),
                    (Some(range), None) | (None, Some(range)) => Ok(range),
                    (None, None) => {
                        let true_range = when_true.value_range(domain)?;
                        let false_range = when_false.value_range(domain)?;
                        Ok(true_range.union(false_range))
                    }
                }
            }
            Self::Binary(Binary::And, left_expression, right) => {
                let left = left_expression.value_range(domain)?;
                if left.is_exact(0) {
                    return Ok(ValueRange::exact(0));
                }
                let right_domain = refine_domain(left_expression, domain, true).unwrap_or(domain);
                right.value_range(right_domain)?;
                Ok(ValueRange { min: 0, max: 1 })
            }
            Self::Binary(Binary::Or, left_expression, right) => {
                let left = left_expression.value_range(domain)?;
                if left.always_nonzero() {
                    return Ok(ValueRange::exact(1));
                }
                let right_domain = refine_domain(left_expression, domain, false).unwrap_or(domain);
                right.value_range(right_domain)?;
                Ok(ValueRange { min: 0, max: 1 })
            }
            Self::Binary(operator, left, right) => {
                let left_range = left.value_range(domain)?;
                let right_range = right.value_range(domain)?;
                match operator {
                    Binary::Eq | Binary::Ne | Binary::Le | Binary::Ge | Binary::Lt | Binary::Gt => {
                        Ok(comparison_range(*operator, left, right))
                    }
                    Binary::Add => ValueRange::from_i128(
                        i128::from(left_range.min) + i128::from(right_range.min),
                        i128::from(left_range.max) + i128::from(right_range.max),
                    ),
                    Binary::Sub => ValueRange::from_i128(
                        i128::from(left_range.min) - i128::from(right_range.max),
                        i128::from(left_range.max) - i128::from(right_range.min),
                    ),
                    Binary::Mul => {
                        let products = [
                            i128::from(left_range.min) * i128::from(right_range.min),
                            i128::from(left_range.min) * i128::from(right_range.max),
                            i128::from(left_range.max) * i128::from(right_range.min),
                            i128::from(left_range.max) * i128::from(right_range.max),
                        ];
                        let min = products
                            .iter()
                            .copied()
                            .min()
                            .unwrap_or(i128::from(i64::MIN));
                        let max = products
                            .iter()
                            .copied()
                            .max()
                            .unwrap_or(i128::from(i64::MAX));
                        ValueRange::from_i128(min, max)
                    }
                    Binary::Div if left == right => {
                        if right_range.contains(0) {
                            Err(PluralRuleError::DivisionByZero)
                        } else {
                            Ok(ValueRange::exact(1))
                        }
                    }
                    Binary::Rem if left == right => {
                        if right_range.contains(0) {
                            Err(PluralRuleError::DivisionByZero)
                        } else {
                            Ok(ValueRange::exact(0))
                        }
                    }
                    Binary::Div => range::division(left_range, right_range),
                    Binary::Rem => range::remainder(left_range, right_range),
                    Binary::And | Binary::Or => unreachable!(
                        "short-circuit operators are handled before arithmetic operators"
                    ),
                }
            }
        }
    }
}
