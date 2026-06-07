use std::str::FromStr;

use crate::{BenchmarkResult, error};

const ENV_SCALES: &str = "RECITE_BENCH_SCALES";
const REALISTIC_V1_PACK: &str = "realistic:v1-pack";
const SCALE_NAMES: &[(BenchmarkScale, &str)] = &[
    (BenchmarkScale::Tiny, "tiny"),
    (BenchmarkScale::Small, "small"),
    (BenchmarkScale::Medium, "medium"),
    (BenchmarkScale::Large, "large"),
    (BenchmarkScale::Epic, "epic"),
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BenchmarkScale {
    Tiny,
    Small,
    Medium,
    Large,
    Epic,
}

impl BenchmarkScale {
    pub const ALL: [Self; 5] = [
        Self::Tiny,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::Epic,
    ];

    pub const DEFAULT: [Self; 2] = [Self::Tiny, Self::Small];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        for (scale, name) in SCALE_NAMES {
            if *scale == self {
                return name;
            }
        }
        unreachable!("every benchmark scale has a string name")
    }

    pub fn selected_from_env() -> BenchmarkResult<Vec<Self>> {
        match std::env::var(ENV_SCALES) {
            Ok(value) => parse_scale_list(&value),
            Err(std::env::VarError::NotPresent) => Ok(Self::DEFAULT.to_vec()),
            Err(var_error) => Err(error(format!(
                "{ENV_SCALES} is not valid UTF-8: {var_error}"
            ))),
        }
    }
}

impl std::fmt::Display for BenchmarkScale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BenchmarkScale {
    type Err = crate::BenchmarkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        SCALE_NAMES
            .iter()
            .find_map(|(scale, name)| (*name == value).then_some(*scale))
            .ok_or_else(|| error(format!("unknown benchmark scale `{value}`")))
    }
}

pub fn parse_scale_list(value: &str) -> BenchmarkResult<Vec<BenchmarkScale>> {
    let mut scales = Vec::new();
    for part in value.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Err(error("benchmark scale list contains an empty entry"));
        }
        let scale = BenchmarkScale::from_str(trimmed)?;
        if !scales.contains(&scale) {
            scales.push(scale);
        }
    }
    if scales.is_empty() {
        return Err(error("benchmark scale list is empty"));
    }
    Ok(scales)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BenchmarkFixture {
    Synthetic(BenchmarkScale),
    RealisticV1Pack,
}

impl BenchmarkFixture {
    pub const DEFAULT: [Self; 2] = [
        Self::Synthetic(BenchmarkScale::Tiny),
        Self::Synthetic(BenchmarkScale::Small),
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic(scale) => scale.as_str(),
            Self::RealisticV1Pack => REALISTIC_V1_PACK,
        }
    }

    #[must_use]
    pub fn asset_stem(self) -> &'static str {
        match self {
            Self::Synthetic(scale) => scale.as_str(),
            Self::RealisticV1Pack => "realistic-v1-pack",
        }
    }

    pub fn selected_from_env() -> BenchmarkResult<Vec<Self>> {
        match std::env::var(ENV_SCALES) {
            Ok(value) => parse_fixture_list(&value),
            Err(std::env::VarError::NotPresent) => Ok(Self::DEFAULT.to_vec()),
            Err(var_error) => Err(error(format!(
                "{ENV_SCALES} is not valid UTF-8: {var_error}"
            ))),
        }
    }
}

impl std::fmt::Display for BenchmarkFixture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BenchmarkFixture {
    type Err = crate::BenchmarkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value == REALISTIC_V1_PACK {
            return Ok(Self::RealisticV1Pack);
        }
        BenchmarkScale::from_str(value).map(Self::Synthetic)
    }
}

pub fn parse_fixture_list(value: &str) -> BenchmarkResult<Vec<BenchmarkFixture>> {
    let mut fixtures = Vec::new();
    for part in value.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Err(error("benchmark fixture list contains an empty entry"));
        }
        let fixture = BenchmarkFixture::from_str(trimmed)?;
        if !fixtures.contains(&fixture) {
            fixtures.push(fixture);
        }
    }
    if fixtures.is_empty() {
        return Err(error("benchmark fixture list is empty"));
    }
    Ok(fixtures)
}
