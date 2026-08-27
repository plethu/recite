use recite_core::CoreValueError;

use super::{
    DialogueChoiceAvailabilityReasonSnapshot, DialogueChoiceAvailabilityReasonTreeSnapshot,
    DialogueChoiceAvailabilitySnapshot,
    conversion::{DialogueSessionSnapshotConversionError, availability_from_snapshot},
};

fn reason_snapshot(id: &str) -> DialogueChoiceAvailabilityReasonSnapshot {
    DialogueChoiceAvailabilityReasonSnapshot {
        id: id.to_owned(),
        source_text: "requires=(trust_gte(hazel, rhea, 3))".to_owned(),
        text: "hazel does not trust rhea enough (3).".to_owned(),
        origin: None,
        args: Vec::new(),
    }
}

#[test]
fn malformed_primary_reason_id_reports_typed_context() {
    let snapshot = DialogueChoiceAvailabilitySnapshot {
        is_available: false,
        primary_reason: Some(reason_snapshot("")),
        reason_tree: None,
    };

    let error = availability_from_snapshot(snapshot).expect_err("empty reason ID is invalid");

    assert!(matches!(
        error,
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id.is_empty()
    ));
}

#[test]
fn malformed_nested_reason_id_reports_typed_context() {
    let snapshot = DialogueChoiceAvailabilitySnapshot {
        is_available: false,
        primary_reason: None,
        reason_tree: Some(DialogueChoiceAvailabilityReasonTreeSnapshot::All(vec![
            DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason_snapshot("  ")),
        ])),
    };

    let error = availability_from_snapshot(snapshot).expect_err("blank reason ID is invalid");

    assert!(matches!(
        error,
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id == "  "
    ));
}
