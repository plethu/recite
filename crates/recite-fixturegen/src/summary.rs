use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::{FixtureConfigSet, FixtureError, FixtureProfile};
use crate::generator::FixtureGenerator;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureSummary {
    pub profile: FixtureProfile,
    pub counts: FixtureCounts,
    pub files: Vec<FileSummary>,
    pub summary_hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureCounts {
    pub blocks: u32,
    pub lines: u32,
    pub choices: u32,
    pub localisable_entries: u32,
    pub generated_words: u32,
    pub shards: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub bytes: u64,
    pub blake3: String,
}

pub struct SummarySet;

impl SummarySet {
    pub fn generate_one(config: &FixtureProfile) -> Result<FixtureSummary, FixtureError> {
        let mut generator = FixtureGenerator::new(config.clone())?;
        generator.emit_project(None)
    }

    pub fn generate_all(configs: &FixtureConfigSet) -> Result<Vec<FixtureSummary>, FixtureError> {
        configs
            .profiles()
            .map(|(_, profile)| Self::generate_one(profile))
            .collect()
    }
}

pub fn write_profile_summary(
    summary: &FixtureSummary,
    output_path: impl AsRef<Path>,
) -> Result<(), FixtureError> {
    let bytes = serde_json::to_vec_pretty(summary).map_err(FixtureError::Json)?;
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(FixtureError::Io)?;
    }
    fs::write(output_path, [bytes, b"\n".to_vec()].concat()).map_err(FixtureError::Io)
}

pub fn write_summaries(
    summaries: &[FixtureSummary],
    output_dir: impl AsRef<Path>,
) -> Result<(), FixtureError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_err(FixtureError::Io)?;
    for summary in summaries {
        write_profile_summary(
            summary,
            output_dir.join(format!("{}.json", summary.profile.name)),
        )?;
    }
    Ok(())
}

pub(crate) fn summary_hash(
    profile: &FixtureProfile,
    counts: &FixtureCounts,
    files: &[FileSummary],
) -> Result<String, FixtureError> {
    #[derive(Serialize)]
    struct HashInput<'a> {
        profile: &'a FixtureProfile,
        counts: &'a FixtureCounts,
        files: &'a [FileSummary],
    }
    let bytes = serde_json::to_vec(&HashInput {
        profile,
        counts,
        files,
    })
    .map_err(FixtureError::Json)?;
    Ok(hash_hex(&bytes))
}

pub(crate) fn hash_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
