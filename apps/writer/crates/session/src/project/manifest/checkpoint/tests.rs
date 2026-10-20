use super::*;

#[test]
fn failed_directory_sync_reports_the_committed_checkpoint_and_installs_its_state() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    let mut owner = ManifestDraft::open(root.path()).unwrap();
    let candidate = "format_version=1\n# renamed\n";
    let outcome = owner
        .checkpoint_with_sync(
            candidate.into(),
            Default::default(),
            vec!["scene.recite".into()],
            |_| Err(io::Error::other("directory unavailable")),
        )
        .unwrap();
    assert!(matches!(outcome, Checkpoint::WrittenWithWarning(_)));
    assert_eq!(owner.text(), candidate);
    assert!(owner.dirty());
    let observed: Draft = serde_json::from_str(&read_regular(&owner.recovery).unwrap()).unwrap();
    assert_eq!(observed.text, candidate);
    drop(owner);
    let recovered = ManifestDraft::open(root.path()).unwrap();
    assert_eq!(recovered.text(), candidate);
}
#[test]
fn failure_before_replacement_preserves_the_old_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    let mut owner = ManifestDraft::open(root.path()).unwrap();
    std::fs::create_dir(&owner.recovery).unwrap();
    assert!(
        owner
            .checkpoint(
                "changed".into(),
                Default::default(),
                vec!["scene.recite".into()]
            )
            .is_err()
    );
    assert_eq!(owner.text(), "format_version=1\n");
}
