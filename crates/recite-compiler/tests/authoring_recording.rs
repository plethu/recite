#![cfg(test)]

use recite_compiler::authoring::{
    ImportRefusal, RecordingItem, RecordingState, handoff_rows, plan_import, recording_items,
    recording_report,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
};
use recite_core::po::PoDocument;
use recite_core::recording::{HandoffRow, RecordingSidecar, SidecarItem, StoredStatus};

const SOURCE: &str = concat!(
    ":: dock default speaker=pilot\n",
    "> tone=\"low\" camera=\"wide\" @dock_greeting\n",
    "  Fog on the river.\n",
    ":if ready()\n",
    "  > speaker=clerk @dock_clerk\n",
    "    Tickets, please.\n",
    "-> END\n",
);

fn compile(source: &str) -> CompiledDialogue {
    let report = compile_inputs(
        [CompileInput::new("scenes/dock.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.1.0").expect("version"),
            CompiledAssetId::new("build/dock.recitec").expect("asset id"),
            SourceMapId::new("build/dock.recitec.map").expect("map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compile does not hard fail");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.asset.expect("asset").dialogue
}

fn items(source: &str, catalog: Option<&PoDocument>) -> Vec<RecordingItem> {
    recording_items(&compile(source), &["tone".to_owned()], catalog)
}

fn state(items: &[RecordingItem], sidecar: &RecordingSidecar, id: &str) -> RecordingState {
    recording_report(items, sidecar)
        .into_iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.state)
        .expect("entry")
}

fn recorded_at(item: &RecordingItem) -> SidecarItem {
    SidecarItem {
        status: StoredStatus::Recorded,
        revision: item.revision.as_ref().map(ToString::to_string),
        note: None,
        takes: Vec::new(),
    }
}

#[test]
fn source_items_follow_statement_order_with_default_speaker_and_declared_direction() {
    let items = items(SOURCE, None);
    assert_eq!(
        items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["dock_greeting", "dock_clerk"]
    );
    let greeting = &items[0];
    assert_eq!(greeting.block, "dock");
    assert_eq!(greeting.file, "scenes/dock.recite");
    assert_eq!(greeting.line, 2);
    assert_eq!(greeting.speaker.as_deref(), Some("pilot"));
    // `camera` is metadata but not a declared direction key.
    assert_eq!(greeting.direction, [("tone".to_owned(), "low".to_owned())]);
    assert_eq!(
        greeting.forms.as_deref(),
        Some(&["Fog on the river.".to_owned()][..])
    );
    assert!(greeting.revision.is_some());
    assert_eq!(items[1].speaker.as_deref(), Some("clerk"));
}

#[test]
fn locale_items_are_ready_only_for_reviewed_translations_of_the_current_source() {
    let catalog = |greeting: &str| {
        PoDocument::parse(format!(
            "msgctxt \"dock_greeting\"\nmsgid \"Fog on the river.\"\n{greeting}\n\n\
             #, fuzzy\nmsgctxt \"dock_clerk\"\nmsgid \"Tickets, please.\"\nmsgstr \"Billets.\"\n"
        ))
        .expect("catalogue")
    };
    let reviewed = items(SOURCE, Some(&catalog("msgstr \"Brume.\"")));
    assert_eq!(
        reviewed[0].forms.as_deref(),
        Some(&["Brume.".to_owned()][..])
    );
    assert!(
        reviewed[1].revision.is_none(),
        "fuzzy translations are not ready"
    );
    assert!(
        items(SOURCE, Some(&catalog("msgstr \"\"")))[0]
            .revision
            .is_none()
    );
    let reworded = SOURCE.replace("Fog on the river.", "Fog on the water.");
    assert!(
        items(&reworded, Some(&catalog("msgstr \"Brume.\"")))[0]
            .revision
            .is_none()
    );
}

#[test]
fn report_derives_every_state_and_speaker_or_direction_changes_make_recordings_stale() {
    let current = items(SOURCE, None);
    let mut sidecar = RecordingSidecar::new(None);
    assert_eq!(
        state(&current, &sidecar, "dock_greeting"),
        RecordingState::ToRecord
    );

    sidecar
        .items
        .insert("dock_greeting".to_owned(), recorded_at(&current[0]));
    assert_eq!(
        state(&current, &sidecar, "dock_greeting"),
        RecordingState::Recorded
    );
    sidecar.items.get_mut("dock_greeting").expect("item").status = StoredStatus::Approved;
    assert_eq!(
        state(&current, &sidecar, "dock_greeting"),
        RecordingState::Approved
    );

    for changed in [
        SOURCE.replace("Fog on the river.", "Fog on the water."),
        SOURCE.replace(
            ":: dock default speaker=pilot",
            ":: dock default speaker=clerk",
        ),
        SOURCE.replace("tone=\"low\"", "tone=\"urgent\""),
    ] {
        let changed = items(&changed, None);
        assert_eq!(
            state(&changed, &sidecar, "dock_greeting"),
            RecordingState::Stale
        );
    }
    // Metadata outside the direction keys is not part of the performance.
    let reframed = items(&SOURCE.replace("camera=\"wide\"", "camera=\"close\""), None);
    assert_eq!(
        state(&reframed, &sidecar, "dock_greeting"),
        RecordingState::Approved
    );

    sidecar.items.insert(
        "dock_clerk".to_owned(),
        SidecarItem {
            status: StoredStatus::Unvoiced,
            revision: None,
            note: None,
            takes: Vec::new(),
        },
    );
    sidecar
        .items
        .insert("removed_line".to_owned(), recorded_at(&current[0]));
    let report = recording_report(&current, &sidecar);
    assert_eq!(report[1].state, RecordingState::Unvoiced);
    assert_eq!(report[2].id, "removed_line");
    assert_eq!(report[2].state, RecordingState::Obsolete);
    assert!(report[2].item.is_none());
}

#[test]
fn import_refuses_every_bad_row_and_applies_valid_rows_once() {
    let current = items(SOURCE, None);
    let sidecar = RecordingSidecar::new(None);
    let mut rows = handoff_rows(&recording_report(&current, &sidecar), None, "session-1");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].direction, "tone=low");
    rows[0].take = "audio/dock_greeting.wav".to_owned();
    rows[0].by = "Studio A".to_owned();
    let exists = |path: &str| path == "audio/dock_greeting.wav";

    let returned = |row: HandoffRow| vec![(2, rows[0].clone()), (3, row)];
    let mut bad = rows[1].clone();
    bad.take = "audio/missing.wav".to_owned();
    for (row, reason) in [
        (
            HandoffRow {
                id: "nobody".to_owned(),
                ..bad.clone()
            },
            ImportRefusal::UnknownId,
        ),
        (
            HandoffRow {
                locale: "fr".to_owned(),
                ..bad.clone()
            },
            ImportRefusal::WrongLocale {
                found: "fr".to_owned(),
            },
        ),
        (rows[0].clone(), ImportRefusal::DuplicateTake),
        (bad.clone(), ImportRefusal::MissingMedia),
        (
            HandoffRow {
                revision: "r1-0000000000000000".to_owned(),
                ..bad.clone()
            },
            ImportRefusal::StaleRevision {
                current: current[1].revision.as_ref().expect("ready").to_string(),
            },
        ),
    ] {
        let refusals =
            plan_import(&current, &sidecar, None, &returned(row), &exists).expect_err("refused");
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0].row, 3);
        assert_eq!(refusals[0].reason, reason);
    }

    // The unrecorded row has no take and is skipped; a URI reference is external media.
    let mut external = rows[1].clone();
    external.take = "https://example.invalid/takes/42".to_owned();
    let plan = plan_import(
        &current,
        &sidecar,
        None,
        &[(2, rows[0].clone()), (3, external)],
        &exists,
    )
    .expect("valid import");
    assert_eq!(plan.added.len(), 2);
    let stored = &plan.sidecar.items["dock_greeting"];
    assert_eq!(stored.status, StoredStatus::Recorded);
    assert_eq!(stored.takes[0].by.as_deref(), Some("Studio A"));
    assert_eq!(stored.takes[0].handoff.as_deref(), Some("session-1"));
    assert_eq!(
        state(&current, &plan.sidecar, "dock_greeting"),
        RecordingState::Recorded
    );

    let again = plan_import(
        &current,
        &plan.sidecar,
        None,
        &[(2, rows[0].clone())],
        &exists,
    )
    .expect("reimport");
    assert!(again.added.is_empty());
    assert_eq!(again.already_present, 1);
    assert_eq!(again.sidecar, plan.sidecar);
}
