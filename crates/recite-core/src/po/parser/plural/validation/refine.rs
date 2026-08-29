use super::super::ast::{Binary, Expr, Unary};
use super::range::{CountDomain, ValueRange};

pub(super) fn comparison_range(operator: Binary, left: &Expr, right: &Expr) -> ValueRange {
    if left == right {
        return match operator {
            Binary::Eq | Binary::Le | Binary::Ge => ValueRange::exact(1),
            Binary::Ne | Binary::Lt | Binary::Gt => ValueRange::exact(0),
            _ => ValueRange { min: 0, max: 1 },
        };
    }
    ValueRange { min: 0, max: 1 }
}

pub(super) fn refine_domain(
    condition: &Expr,
    domain: CountDomain,
    desired: bool,
) -> Option<CountDomain> {
    if let Some(known) = known_condition(condition) {
        return (known == desired).then_some(domain);
    }
    let Expr::Binary(operator, left, right) = condition else {
        return None;
    };
    let (operator, value) = count_comparison(*operator, left, right)?;
    let operator = if desired {
        operator
    } else {
        negate_comparison(operator)
    };
    match operator {
        Binary::Eq => {
            (domain.min <= value && value <= domain.max).then_some(CountDomain::exact(value))
        }
        Binary::Ne => exclude_value(domain, value),
        Binary::Le => upper_bound(domain, value),
        Binary::Ge => lower_bound(domain, value),
        Binary::Lt => value
            .checked_sub(1)
            .and_then(|max| upper_bound(domain, max)),
        Binary::Gt => value
            .checked_add(1)
            .and_then(|min| lower_bound(domain, min)),
        _ => None,
    }
}

fn exclude_value(domain: CountDomain, value: i64) -> Option<CountDomain> {
    if domain.min == domain.max && domain.min == value {
        None
    } else if value == domain.min {
        value.checked_add(1).map(|min| CountDomain {
            min,
            max: domain.max,
        })
    } else if value == domain.max {
        value.checked_sub(1).map(|max| CountDomain {
            min: domain.min,
            max,
        })
    } else {
        Some(domain)
    }
}

fn upper_bound(domain: CountDomain, max: i64) -> Option<CountDomain> {
    (max >= domain.min).then_some(CountDomain {
        min: domain.min,
        max: domain.max.min(max),
    })
}

fn lower_bound(domain: CountDomain, min: i64) -> Option<CountDomain> {
    (min <= domain.max).then_some(CountDomain {
        min: domain.min.max(min),
        max: domain.max,
    })
}

const fn negate_comparison(operator: Binary) -> Binary {
    match operator {
        Binary::Eq => Binary::Ne,
        Binary::Ne => Binary::Eq,
        Binary::Le => Binary::Gt,
        Binary::Ge => Binary::Lt,
        Binary::Lt => Binary::Ge,
        Binary::Gt => Binary::Le,
        _ => operator,
    }
}

fn known_condition(expression: &Expr) -> Option<bool> {
    let Expr::Binary(operator, left, right) = expression else {
        return None;
    };
    if !matches!(
        operator,
        Binary::Eq | Binary::Ne | Binary::Le | Binary::Ge | Binary::Lt | Binary::Gt
    ) {
        return None;
    }
    (left == right).then(|| match operator {
        Binary::Eq | Binary::Le | Binary::Ge => true,
        Binary::Ne | Binary::Lt | Binary::Gt => false,
        _ => unreachable!("known conditions are comparison expressions"),
    })
}

fn count_comparison(operator: Binary, left: &Expr, right: &Expr) -> Option<(Binary, i64)> {
    match (left, right) {
        (Expr::N, literal) => integer_literal(literal).map(|value| (operator, value)),
        (literal, Expr::N) => {
            integer_literal(literal).map(|value| (reverse_comparison(operator), value))
        }
        _ => None,
    }
}

fn integer_literal(expression: &Expr) -> Option<i64> {
    match expression {
        Expr::Number(value) => Some(*value),
        Expr::Unary(Unary::Plus, expression) => integer_literal(expression),
        Expr::Unary(Unary::Minus, expression) => {
            integer_literal(expression).and_then(i64::checked_neg)
        }
        _ => None,
    }
}

const fn reverse_comparison(operator: Binary) -> Binary {
    match operator {
        Binary::Le => Binary::Ge,
        Binary::Ge => Binary::Le,
        Binary::Lt => Binary::Gt,
        Binary::Gt => Binary::Lt,
        _ => operator,
    }
}
