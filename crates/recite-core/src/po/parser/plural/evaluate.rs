use super::PluralRuleError;
use super::ast::{Binary, Expr, Unary};

impl Expr {
    pub(super) fn evaluate(&self, n: i64) -> Result<i64, PluralRuleError> {
        match self {
            Self::Number(value) => Ok(*value),
            Self::N => Ok(n),
            Self::Unary(Unary::Not, expression) => Ok((expression.evaluate(n)? == 0) as i64),
            Self::Unary(Unary::Plus, expression) => expression.evaluate(n),
            Self::Unary(Unary::Minus, expression) => expression
                .evaluate(n)?
                .checked_neg()
                .ok_or(PluralRuleError::ArithmeticOverflow),
            Self::Conditional(condition, when_true, when_false) => {
                if condition.evaluate(n)? != 0 {
                    when_true.evaluate(n)
                } else {
                    when_false.evaluate(n)
                }
            }
            Self::Binary(Binary::And, left, right) => {
                if left.evaluate(n)? == 0 {
                    Ok(0)
                } else {
                    Ok((right.evaluate(n)? != 0) as i64)
                }
            }
            Self::Binary(Binary::Or, left, right) => {
                if left.evaluate(n)? != 0 {
                    Ok(1)
                } else {
                    Ok((right.evaluate(n)? != 0) as i64)
                }
            }
            Self::Binary(operator, left, right) => {
                let left = left.evaluate(n)?;
                let right = right.evaluate(n)?;
                let result = match operator {
                    Binary::Eq => Some((left == right) as i64),
                    Binary::Ne => Some((left != right) as i64),
                    Binary::Le => Some((left <= right) as i64),
                    Binary::Ge => Some((left >= right) as i64),
                    Binary::Lt => Some((left < right) as i64),
                    Binary::Gt => Some((left > right) as i64),
                    Binary::Add => left.checked_add(right),
                    Binary::Sub => left.checked_sub(right),
                    Binary::Mul => left.checked_mul(right),
                    Binary::Div => {
                        if right == 0 {
                            return Err(PluralRuleError::DivisionByZero);
                        }
                        left.checked_div(right)
                    }
                    Binary::Rem => {
                        if right == 0 {
                            return Err(PluralRuleError::DivisionByZero);
                        }
                        left.checked_rem(right)
                    }
                    Binary::And | Binary::Or => unreachable!(),
                };
                result.ok_or(PluralRuleError::ArithmeticOverflow)
            }
        }
    }
}
