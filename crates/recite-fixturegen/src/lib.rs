//! Deterministic synthetic fixture generation for Recite benchmark profiles.
//!
//! This crate is tooling support for generating repeatable Recite projects used
//! by benchmarks and scale tests. The generated projects include source shards,
//! schema manifests, locale catalogs, project manifests, runtime fixtures, and
//! summary metadata.
//!
//! It is not part of the dialogue runtime contract. Use it when a test or
//! benchmark needs larger deterministic projects instead of hand-maintained
//! fixtures.
//!
//! # Example
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_fixturegen::{FixtureProfile, generate_tiny_in_memory};
//!
//! let profile = FixtureProfile {
//!     name: "docs".to_owned(),
//!     seed: 7,
//!     blocks: 1,
//!     lines: 1,
//!     choices: 1,
//!     localisable_entries: 2,
//!     generated_words: 8,
//!     shards: 1,
//! };
//!
//! let project = generate_tiny_in_memory(&profile)?;
//! assert!(project.files.contains_key("recite.project.toml"));
//! assert!(project.files.contains_key("src/shard-000.recite"));
//! assert_eq!(project.summary.profile.name, "docs");
//! # Ok(())
//! # }
//! ```

mod config;
mod content;
mod generator;
mod summary;

pub use config::{FixtureConfigSet, FixtureError, FixtureProfile, GenerateMode};
pub use generator::{GeneratedProject, generate_tiny_in_memory, write_project};
pub use summary::{
    FileSummary, FixtureCounts, FixtureSummary, SummarySet, write_profile_summary, write_summaries,
};
