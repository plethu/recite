use recite_compiler::authoring::{SchemaFreshness, SchemaFreshnessUnavailableReason};
use recite_core::compiled::ContentFingerprint;
use recite_core::schema::{
    ContentFingerprintFreshness, ProducerFingerprint, ProducerFingerprintMismatch,
    ProducerFreshness, SchemaProducerFreshness,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use super::{content_freshness_json, freshness_json, producer_freshness_json};

#[test]
fn unavailable_freshness_names_why_comparison_is_absent() {
    for (reason, expected) in [
        (
            SchemaFreshnessUnavailableReason::NoComparisonSnapshot,
            "no_comparison_snapshot",
        ),
        (
            SchemaFreshnessUnavailableReason::NoProducerMetadata,
            "no_producer_metadata",
        ),
    ] {
        assert_eq!(
            project(SchemaFreshness::Unavailable { reason }),
            json!({"status":"unavailable","reason":expected,"channels":null}),
        );
    }
}

#[test]
fn content_evidence_projects_missing_mismatch_and_unexpected_digests() {
    let expected = digest(0xab);
    let actual = digest(0x12);
    let expected_json = digest_json("ab");
    let actual_json = digest_json("12");
    for (evidence, projected, status) in [
        (
            ContentFingerprintFreshness::Fresh,
            json!({"status":"fresh"}),
            "fresh",
        ),
        (
            ContentFingerprintFreshness::Missing {
                expected: expected.clone(),
            },
            json!({"status":"missing","expected":expected_json}),
            "missing",
        ),
        (
            ContentFingerprintFreshness::Mismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            },
            json!({"status":"mismatch","expected":expected_json,"actual":actual_json}),
            "mismatch",
        ),
        (
            ContentFingerprintFreshness::Unexpected { actual },
            json!({"status":"unexpected","actual":actual_json}),
            "unexpected",
        ),
    ] {
        assert_eq!(content_freshness_json(&evidence), projected);
        let mut comparison = fresh();
        comparison.content_fingerprint = evidence;
        let result = project(SchemaFreshness::Compared(Box::new(comparison)));
        assert_eq!(result["status"], status);
        assert_eq!(result["channels"]["content"], projected);
        assert_eq!(result["reason"], Value::Null);
    }
}

#[test]
fn producer_evidence_preserves_typed_inputs_and_both_mismatch_sides() {
    let expected = producer("input", "expected");
    let actual = producer("input", "actual");
    let extra = producer("extra", "unexpected");
    let mismatch = ProducerFingerprintMismatch {
        expected: expected.clone(),
        actual: actual.clone(),
    };
    let expected_json = producer_json("input", "expected");
    let actual_json = producer_json("input", "actual");
    let extra_json = producer_json("extra", "unexpected");
    let mismatch_json = json!({"expected":expected_json,"actual":actual_json});
    let cases = [
        (ProducerFreshness::Fresh, json!({"status":"fresh"}), "fresh"),
        (
            ProducerFreshness::ContentMissing {
                expected: digest(0xab),
            },
            json!({"status":"missing","expected":digest_json("ab")}),
            "missing",
        ),
        (
            ProducerFreshness::ContentMismatch {
                expected: digest(0xab),
                actual: digest(0x12),
            },
            json!({"status":"mismatch","expected":digest_json("ab"),"actual":digest_json("12")}),
            "mismatch",
        ),
        (
            ProducerFreshness::ContentUnexpected {
                actual: digest(0x12),
            },
            json!({"status":"unexpected","actual":digest_json("12")}),
            "unexpected",
        ),
        (
            ProducerFreshness::Invalid {
                expected_duplicates: vec![("file".into(), "input".into())],
                actual_duplicates: vec![("file".into(), "extra".into())],
            },
            json!({"status":"invalid","expected_duplicates":[["file","input"]],"actual_duplicates":[["file","extra"]]}),
            "invalid",
        ),
        (
            ProducerFreshness::Missing {
                expected: vec![expected.clone()],
            },
            json!({"status":"missing","expected":[expected_json]}),
            "missing",
        ),
        (
            ProducerFreshness::Mismatch {
                entries: vec![mismatch.clone()],
            },
            json!({"status":"mismatch","entries":[mismatch_json]}),
            "mismatch",
        ),
        (
            ProducerFreshness::Unexpected {
                actual: vec![extra.clone()],
            },
            json!({"status":"unexpected","actual":[extra_json]}),
            "unexpected",
        ),
        (
            ProducerFreshness::Mixed {
                missing: vec![expected],
                mismatched: vec![mismatch],
                unexpected: vec![extra],
            },
            json!({"status":"mixed","missing":[expected_json],"mismatched":[mismatch_json],"unexpected":[extra_json]}),
            "mixed",
        ),
    ];
    for (evidence, projected, status) in cases {
        assert_eq!(producer_freshness_json(&evidence), projected);
        let mut comparison = fresh();
        comparison.manifest = evidence.clone();
        comparison
            .registries
            .insert("speaker_registry".into(), evidence.clone());
        comparison
            .metadata_domains
            .insert("quests".into(), evidence);
        assert_eq!(
            project(SchemaFreshness::Compared(Box::new(comparison))),
            json!({"status":status,"reason":null,"channels":{
                "content":{"status":"fresh"},"manifest":projected,
                "registries":{"speaker_registry":projected},"metadata_domains":{"quests":projected},
            }}),
        );
    }
}

#[test]
fn aggregate_status_combines_channel_failures_and_prioritizes_invalid_evidence() {
    let mut comparison = fresh();
    comparison.content_fingerprint = ContentFingerprintFreshness::Missing {
        expected: digest(0xab),
    };
    comparison.registries.insert(
        "speakers".into(),
        ProducerFreshness::Mismatch { entries: vec![] },
    );
    comparison.metadata_domains.insert(
        "quests".into(),
        ProducerFreshness::Unexpected { actual: vec![] },
    );
    let combined = project(SchemaFreshness::Compared(Box::new(comparison.clone())));
    assert_eq!(combined["status"], "mixed");
    assert_eq!(
        combined["channels"]["registries"]["speakers"],
        json!({"status":"mismatch","entries":[]})
    );
    assert_eq!(
        combined["channels"]["metadata_domains"]["quests"],
        json!({"status":"unexpected","actual":[]})
    );
    comparison.manifest = ProducerFreshness::Invalid {
        expected_duplicates: vec![],
        actual_duplicates: vec![("file".into(), "input".into())],
    };
    let invalid = project(SchemaFreshness::Compared(Box::new(comparison)));
    assert_eq!(invalid["status"], "invalid");
    assert_eq!(
        invalid["channels"]["manifest"],
        json!({"status":"invalid","expected_duplicates":[],"actual_duplicates":[["file","input"]]})
    );
}

fn fresh() -> SchemaProducerFreshness {
    SchemaProducerFreshness {
        content_fingerprint: ContentFingerprintFreshness::Fresh,
        manifest: ProducerFreshness::Fresh,
        registries: BTreeMap::new(),
        metadata_domains: BTreeMap::new(),
    }
}

fn project(freshness: SchemaFreshness) -> Value {
    serde_json::to_value(freshness_json(&freshness)).unwrap()
}

fn digest(value: u8) -> ContentFingerprint {
    ContentFingerprint::blake3(vec![value; 32]).unwrap()
}

fn digest_json(value: &str) -> Value {
    json!({"algorithm":"blake3","value":value.repeat(32)})
}

fn producer(id: &str, value: &str) -> ProducerFingerprint {
    ProducerFingerprint {
        kind: "file".into(),
        id: id.into(),
        algorithm: "sha256".into(),
        value: value.into(),
    }
}

fn producer_json(id: &str, value: &str) -> Value {
    json!({"kind":"file","id":id,"algorithm":"sha256","value":value})
}
