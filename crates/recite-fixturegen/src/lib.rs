//! Deterministic synthetic fixture generation for Recite benchmark profiles.

mod config;
mod content;
mod generator;
mod summary;

pub use config::{FixtureConfigSet, FixtureError, FixtureProfile, GenerateMode};
pub use generator::{GeneratedProject, generate_tiny_in_memory, write_project};
pub use summary::{
    FileSummary, FixtureCounts, FixtureSummary, SummarySet, write_profile_summary, write_summaries,
};
