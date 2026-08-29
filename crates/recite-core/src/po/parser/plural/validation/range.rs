use super::super::PluralRuleError;

#[derive(Clone, Copy, Debug)]
pub(super) struct ValueRange {
    pub(super) min: i64,
    pub(super) max: i64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CountDomain {
    pub(super) min: i64,
    pub(super) max: i64,
}

impl CountDomain {
    pub(super) const ALL: Self = Self {
        min: 0,
        max: i64::MAX,
    };

    pub(super) const fn exact(value: i64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }
}

impl ValueRange {
    pub(super) const fn from_domain(domain: CountDomain) -> Self {
        Self {
            min: domain.min,
            max: domain.max,
        }
    }

    pub(super) const fn exact(value: i64) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    pub(super) fn from_i128(min: i128, max: i128) -> Result<Self, PluralRuleError> {
        if min < i128::from(i64::MIN) || max > i128::from(i64::MAX) || min > max {
            return Err(PluralRuleError::ArithmeticOverflow);
        }
        Ok(Self {
            min: min as i64,
            max: max as i64,
        })
    }

    pub(super) const fn contains(self, value: i64) -> bool {
        self.min <= value && value <= self.max
    }

    pub(super) const fn is_exact(self, value: i64) -> bool {
        self.min == value && self.max == value
    }

    pub(super) const fn always_nonzero(self) -> bool {
        self.min > 0 || self.max < 0
    }

    pub(super) fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

pub(super) fn division(left: ValueRange, right: ValueRange) -> Result<ValueRange, PluralRuleError> {
    if right.contains(0) {
        return Err(PluralRuleError::DivisionByZero);
    }
    if left.contains(i64::MIN) && right.contains(-1) {
        return Err(PluralRuleError::ArithmeticOverflow);
    }
    let quotients = [
        i128::from(left.min) / i128::from(right.min),
        i128::from(left.min) / i128::from(right.max),
        i128::from(left.max) / i128::from(right.min),
        i128::from(left.max) / i128::from(right.max),
    ];
    let min = quotients
        .iter()
        .copied()
        .min()
        .unwrap_or(i128::from(i64::MIN));
    let max = quotients
        .iter()
        .copied()
        .max()
        .unwrap_or(i128::from(i64::MAX));
    ValueRange::from_i128(min, max)
}

pub(super) fn remainder(
    left: ValueRange,
    right: ValueRange,
) -> Result<ValueRange, PluralRuleError> {
    if right.contains(0) {
        return Err(PluralRuleError::DivisionByZero);
    }
    if left.contains(i64::MIN) && right.contains(-1) {
        return Err(PluralRuleError::ArithmeticOverflow);
    }
    let max_abs_divisor = i128::from(right.min).abs().max(i128::from(right.max).abs());
    let max_abs_remainder = max_abs_divisor - 1;
    let min_abs_remainder = i128::from(left.min).abs().min(max_abs_remainder);
    let max_abs_remainder = i128::from(left.max).min(max_abs_remainder);
    let min = if left.min >= 0 { 0 } else { -min_abs_remainder };
    let max = if left.max <= 0 { 0 } else { max_abs_remainder };
    ValueRange::from_i128(min, max)
}
