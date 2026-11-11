use recite_core::recording::{
    HandoffError, HandoffRow, RecordingRevision, RecordingSidecar, SidecarError, SidecarItem,
    StoredStatus, Take,
};

fn direction(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn revision_changes_with_text_speaker_and_direction_only() {
    let base = RecordingRevision::of(&["Fog."], Some("pilot"), &direction(&[("tone", "low")]));
    assert!(RecordingRevision::parse(base.as_str()).is_some());
    assert!(base.as_str().starts_with("r1-") && base.as_str().len() == 19);
    assert_eq!(
        base,
        RecordingRevision::of(&["Fog."], Some("pilot"), &direction(&[("tone", "low")]))
    );
    for changed in [
        RecordingRevision::of(&["Fog!"], Some("pilot"), &direction(&[("tone", "low")])),
        RecordingRevision::of(&["Fog."], Some("clerk"), &direction(&[("tone", "low")])),
        RecordingRevision::of(&["Fog."], None, &direction(&[("tone", "low")])),
        RecordingRevision::of(&["Fog."], Some("pilot"), &direction(&[("tone", "high")])),
        RecordingRevision::of(
            &["Fog.", "Fogs."],
            Some("pilot"),
            &direction(&[("tone", "low")]),
        ),
    ] {
        assert_ne!(base, changed);
    }
    assert!(RecordingRevision::parse("r1-XYZ").is_none());
    assert!(RecordingRevision::parse("r2-0123456789abcdef").is_none());
}

#[test]
fn sidecar_round_trips_deterministically_and_refuses_bad_state() -> Result<(), SidecarError> {
    let revision = RecordingRevision::of(&["Fog."], Some("pilot"), &[]).to_string();
    let mut sidecar = RecordingSidecar::new(Some("fr".to_owned()));
    sidecar.items.insert(
        "dock_greeting".to_owned(),
        SidecarItem {
            status: StoredStatus::Recorded,
            revision: Some(revision.clone()),
            note: None,
            takes: vec![Take {
                reference: "audio/fr/dock_greeting.wav".to_owned(),
                revision: revision.clone(),
                handoff: Some("fr-1".to_owned()),
                by: Some("Studio A".to_owned()),
                date: None,
            }],
        },
    );
    sidecar.items.insert(
        "arrival_line".to_owned(),
        SidecarItem {
            status: StoredStatus::Unvoiced,
            revision: None,
            note: Some("Ambient caption only".to_owned()),
            takes: Vec::new(),
        },
    );
    let text = sidecar.to_toml_string();
    assert!(text.starts_with("format_version = 1\nlocale = \"fr\"\n"));
    assert!(text.find("[items.arrival_line]") < text.find("[items.dock_greeting]"));
    assert_eq!(RecordingSidecar::parse(&text)?, sidecar);
    assert_eq!(RecordingSidecar::parse(&text)?.to_toml_string(), text);

    assert!(matches!(
        RecordingSidecar::parse("format_version = 2\n"),
        Err(SidecarError::FormatVersion(2))
    ));
    assert!(matches!(
        RecordingSidecar::parse("format_version = 1\n[items.a]\nstatus = \"recorded\"\n"),
        Err(SidecarError::MissingRevision { .. })
    ));
    assert!(matches!(
        RecordingSidecar::parse("format_version = 1\n[items.a]\nstatus = \"heard\"\n"),
        Err(SidecarError::Toml(_))
    ));
    Ok(())
}

#[test]
fn csv_round_trips_awkward_text_and_ignores_extra_columns() -> Result<(), HandoffError> {
    let row = HandoffRow {
        id: "dock_greeting".to_owned(),
        locale: "fr".to_owned(),
        line: 4,
        speaker: "pilot".to_owned(),
        direction: "tone=low; camera=wide".to_owned(),
        source_text: "Fog, \"again\".".to_owned(),
        text: "Encore la brume,\nencore.".to_owned(),
        revision: "r1-0123456789abcdef".to_owned(),
        note: " leading space".to_owned(),
        ..HandoffRow::default()
    };
    let csv = HandoffRow::to_csv(std::slice::from_ref(&row));
    assert!(csv.starts_with("id,locale,handoff,block,file,line,"));
    assert!(csv.contains("\"Fog, \"\"again\"\".\""));
    let parsed = HandoffRow::from_csv(&csv)?;
    assert_eq!(parsed, vec![(2, row.clone())]);

    let edited =
        "\u{feff}take,id,Reviewer,locale,revision\nwav/1.wav,dock_greeting,Sam,fr,r1-x\n\n";
    let parsed = HandoffRow::from_csv(edited)?;
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].1.take, "wav/1.wav");
    assert_eq!(parsed[0].1.id, "dock_greeting");

    assert!(matches!(
        HandoffRow::from_csv("id,locale,revision\n"),
        Err(HandoffError::MissingColumn("take"))
    ));
    assert!(matches!(
        HandoffRow::from_csv("id,locale,revision,take\n\"open,a,b,c\n"),
        Err(HandoffError::Csv { .. })
    ));
    Ok(())
}

#[test]
fn json_round_trips_rows() -> Result<(), HandoffError> {
    let row = HandoffRow {
        id: "deck_crossing".to_owned(),
        take: "audio/deck.wav".to_owned(),
        ..HandoffRow::default()
    };
    let json = HandoffRow::to_json(std::slice::from_ref(&row));
    assert_eq!(HandoffRow::from_json(&json)?, vec![(1, row)]);
    assert!(HandoffRow::from_json("{\"format_version\": 2, \"rows\": []}").is_err());
    Ok(())
}
