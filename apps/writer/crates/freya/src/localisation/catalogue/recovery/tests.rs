use super::*;

#[path = "../performance/tests.rs"]
mod performance;
const SOURCE: &str = "msgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"Bye {name}\"\nmsgstr \"Adieu {name}\"\n";

#[test]
fn snapshots_share_baseline_and_drafts_but_keep_old_values_after_editing()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: false,
        },
    );
    let first = catalogue.recovery_snapshot().ok_or("snapshot")?;
    let second = catalogue.recovery_snapshot().ok_or("snapshot")?;
    assert!(std::sync::Arc::ptr_eq(&first.source, &second.source));
    assert!(std::sync::Arc::ptr_eq(
        &first.drafts[0].1,
        &second.drafts[0].1
    ));
    let cloned = first.clone();
    assert!(std::sync::Arc::ptr_eq(&first.drafts, &cloned.drafts));
    let old_json = serde_json::json!({"version":1,"source":SOURCE,"drafts":[[0,{"forms":["Salut {name}"],"reviewed":false}]]});
    let decoded: SnapshotData = serde_json::from_value(old_json.clone())?;
    assert_eq!(serde_json::to_value(&first)?, old_json);
    assert!(first == decoded);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Bonsoir {name}".into()],
            reviewed: true,
        },
    );
    assert_eq!(first.drafts[0].1.forms, ["Salut {name}"]);
    assert_eq!(
        catalogue.draft(id).ok_or("draft")?.forms,
        ["Bonsoir {name}"]
    );
    Ok(())
}

#[test]
fn partial_save_installs_new_recovery_baseline_for_remaining_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let mut catalogue = Catalogue::open_recoverable(&path)?;
    for i in 0..2 {
        catalogue.update(
            PoEntryId::new(i),
            Draft {
                forms: vec![format!("Updated {i} {{name}}")],
                reviewed: true,
            },
        );
    }
    catalogue.save(PoEntryId::new(0))?;
    let saved = std::fs::read_to_string(&path)?;
    let snapshot = catalogue.recovery_snapshot().ok_or("remaining snapshot")?;
    assert_eq!(snapshot.source.as_str(), saved);
    assert_eq!(snapshot.drafts.len(), 1);
    assert_eq!(snapshot.drafts[0].0, 1);
    drop(catalogue);
    let mut restored = Catalogue::open_recoverable(&path)?;
    assert!(!restored.changed(PoEntryId::new(0)));
    assert_eq!(
        restored
            .draft(PoEntryId::new(1))
            .ok_or("remaining draft")?
            .forms,
        ["Updated 1 {name}"]
    );
    restored.save_all()?;
    assert!(!restored.dirty());
    assert!(std::fs::read_to_string(path)?.contains("Updated 1 {name}"));
    Ok(())
}
