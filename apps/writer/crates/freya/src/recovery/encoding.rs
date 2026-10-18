//! Reuse immutable JSON strings while retaining the version-1 recovery wire format.
use super::Recovery;
use recite_writer_model::{DraftSnapshot, View};
use serde::Serialize;
use serde_json::value::{RawValue, to_raw_value};
use std::{io::Write, sync::Arc};

struct EncodedText {
    source: Arc<str>,
    json: Arc<RawValue>,
}
impl EncodedText {
    fn prepare(
        source: Arc<str>,
        previous: Option<&Self>,
        shared: Option<&Self>,
    ) -> Result<Self, serde_json::Error> {
        for cached in [previous, shared].into_iter().flatten() {
            if Arc::ptr_eq(&source, &cached.source) || source.as_ref() == cached.source.as_ref() {
                return Ok(Self {
                    source,
                    json: Arc::clone(&cached.json),
                });
            }
        }
        let json = to_raw_value(&source)?.into();
        Ok(Self { source, json })
    }
}

struct Sources {
    baseline: EncodedText,
    applied: EncodedText,
}

#[derive(Default)]
pub(crate) struct Encoding {
    sources: Option<Sources>,
}

#[derive(Serialize)]
struct DraftWire<'a> {
    source: &'a RawValue,
    view: &'a View,
    draft: &'a DraftSnapshot,
}
#[derive(Serialize)]
struct RecoveryWire<'a> {
    version: u32,
    baseline: &'a RawValue,
    draft: DraftWire<'a>,
}

impl Encoding {
    pub(super) fn write(
        &mut self,
        snapshot: &Recovery,
        output: impl Write,
    ) -> Result<(), serde_json::Error> {
        let previous = self.sources.as_ref();
        let baseline = EncodedText::prepare(
            Arc::clone(&snapshot.baseline),
            previous.map(|sources| &sources.baseline),
            previous.map(|sources| &sources.applied),
        )?;
        let applied = EncodedText::prepare(
            snapshot.draft.source_snapshot(),
            previous.map(|sources| &sources.applied),
            Some(&baseline),
        )?;
        // Identity and encoded bytes move together. This cache describes the
        // input, independently of whether the following disk write succeeds.
        let sources = self.sources.insert(Sources { baseline, applied });
        serde_json::to_writer(
            output,
            &RecoveryWire {
                version: snapshot.version,
                baseline: &sources.baseline.json,
                draft: DraftWire {
                    source: &sources.applied.json,
                    view: snapshot.draft.view(),
                    draft: snapshot.draft.draft_snapshot(),
                },
            },
        )
    }
}

#[cfg(test)]
mod tests;
