use std::io::Write;

use crate::args::ProducerFreshnessArgs;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::load_schema_for_freshness;
use crate::i18n::Messages;
use recite_core::{
    ContentFingerprintFreshness, ProducerFingerprint, ProducerFingerprintMismatch,
    ProducerFreshness, SchemaProducerFreshness, compare_schema_producer_freshness_detailed,
};

pub(crate) fn check(
    args: ProducerFreshnessArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let expected = load_schema_for_freshness(&args.expected)?;
    let actual = load_schema_for_freshness(&args.actual)?;
    if !expected.diagnostics.is_empty() || !actual.diagnostics.is_empty() {
        report_diagnostics(
            stderr,
            messages,
            expected.diagnostics.iter().chain(actual.diagnostics.iter()),
        )?;
        return Err(CliError::Diagnostics);
    }
    let Some(expected) = expected.schema.as_ref() else {
        return Err(CliError::Diagnostics);
    };
    let Some(actual) = actual.schema.as_ref() else {
        return Err(CliError::Diagnostics);
    };
    let evidence = compare_schema_producer_freshness_detailed(expected, actual);
    let (status, details) = freshness_json(&evidence);
    writeln!(
        stdout,
        "{}",
        serde_json::json!({ "status": status, "evidence": details })
    )?;
    if evidence.is_fresh() {
        Ok(())
    } else {
        Err(CliError::Diagnostics)
    }
}

fn freshness_json(evidence: &SchemaProducerFreshness) -> (&'static str, serde_json::Value) {
    let status = freshness_status(evidence);
    let details = serde_json::json!({
        "content_fingerprint": content_fingerprint_json(&evidence.content_fingerprint),
        "manifest": producer_freshness_json(&evidence.manifest),
        "registries": evidence
            .registries
            .iter()
            .map(|(name, freshness)| (name.clone(), producer_freshness_json(freshness)))
            .collect::<serde_json::Map<_, _>>(),
        "metadata_domains": evidence
            .metadata_domains
            .iter()
            .map(|(name, freshness)| (name.clone(), producer_freshness_json(freshness)))
            .collect::<serde_json::Map<_, _>>(),
    });
    (status, details)
}

fn freshness_status(evidence: &SchemaProducerFreshness) -> &'static str {
    let mut invalid = false;
    let mut missing = false;
    let mut mismatch = false;
    let mut unexpected = false;
    let mut record = |freshness: &ProducerFreshness| match freshness {
        ProducerFreshness::Fresh => {}
        ProducerFreshness::ContentMissing { .. } => missing = true,
        ProducerFreshness::ContentMismatch { .. } => mismatch = true,
        ProducerFreshness::ContentUnexpected { .. } => unexpected = true,
        ProducerFreshness::Invalid { .. } => invalid = true,
        ProducerFreshness::Missing { .. } => missing = true,
        ProducerFreshness::Mismatch { .. } => mismatch = true,
        ProducerFreshness::Unexpected { .. } => unexpected = true,
        ProducerFreshness::Mixed { .. } => {
            missing = true;
            mismatch = true;
            unexpected = true;
        }
    };
    record(&evidence.manifest);
    for freshness in evidence
        .registries
        .values()
        .chain(evidence.metadata_domains.values())
    {
        record(freshness);
    }
    match &evidence.content_fingerprint {
        ContentFingerprintFreshness::Fresh => {}
        ContentFingerprintFreshness::Missing { .. } => missing = true,
        ContentFingerprintFreshness::Mismatch { .. } => mismatch = true,
        ContentFingerprintFreshness::Unexpected { .. } => unexpected = true,
    }
    if invalid {
        "invalid"
    } else if [missing, mismatch, unexpected]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
    {
        "mixed"
    } else if missing {
        "missing"
    } else if mismatch {
        "mismatch"
    } else if unexpected {
        "unexpected"
    } else {
        "fresh"
    }
}

fn producer_freshness_json(evidence: &ProducerFreshness) -> serde_json::Value {
    match evidence {
        ProducerFreshness::Fresh => serde_json::Value::Null,
        ProducerFreshness::ContentMissing { expected } => {
            serde_json::json!({ "expected": content_fingerprint(expected) })
        }
        ProducerFreshness::ContentMismatch { expected, actual } => {
            serde_json::json!({ "expected": content_fingerprint(expected), "actual": content_fingerprint(actual) })
        }
        ProducerFreshness::ContentUnexpected { actual } => {
            serde_json::json!({ "actual": content_fingerprint(actual) })
        }
        ProducerFreshness::Invalid {
            expected_duplicates,
            actual_duplicates,
        } => {
            serde_json::json!({ "expected_duplicates": expected_duplicates, "actual_duplicates": actual_duplicates })
        }
        ProducerFreshness::Missing { expected } => {
            serde_json::json!({ "expected": expected.iter().map(fingerprint_json).collect::<Vec<_>>() })
        }
        ProducerFreshness::Mismatch { entries } => {
            serde_json::json!({ "entries": entries.iter().map(mismatch_json).collect::<Vec<_>>() })
        }
        ProducerFreshness::Unexpected { actual } => {
            serde_json::json!({ "actual": actual.iter().map(fingerprint_json).collect::<Vec<_>>() })
        }
        ProducerFreshness::Mixed {
            missing,
            mismatched,
            unexpected,
        } => {
            serde_json::json!({ "missing": missing.iter().map(fingerprint_json).collect::<Vec<_>>(), "mismatched": mismatched.iter().map(mismatch_json).collect::<Vec<_>>(), "unexpected": unexpected.iter().map(fingerprint_json).collect::<Vec<_>>() })
        }
    }
}

fn content_fingerprint_json(evidence: &ContentFingerprintFreshness) -> serde_json::Value {
    match evidence {
        ContentFingerprintFreshness::Fresh => serde_json::Value::Null,
        ContentFingerprintFreshness::Missing { expected } => {
            serde_json::json!({ "expected": content_fingerprint(expected) })
        }
        ContentFingerprintFreshness::Mismatch { expected, actual } => {
            serde_json::json!({ "expected": content_fingerprint(expected), "actual": content_fingerprint(actual) })
        }
        ContentFingerprintFreshness::Unexpected { actual } => {
            serde_json::json!({ "actual": content_fingerprint(actual) })
        }
    }
}

fn fingerprint_json(value: &ProducerFingerprint) -> serde_json::Value {
    serde_json::json!({ "kind": value.kind, "id": value.id, "algorithm": value.algorithm, "value": value.value })
}

fn mismatch_json(value: &ProducerFingerprintMismatch) -> serde_json::Value {
    serde_json::json!({ "expected": fingerprint_json(&value.expected), "actual": fingerprint_json(&value.actual) })
}

fn content_fingerprint(value: &recite_core::ContentFingerprint) -> serde_json::Value {
    serde_json::json!({
        "algorithm": value.algorithm().as_str(),
        "value": value.digest().as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
    })
}
