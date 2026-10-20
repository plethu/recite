use super::*;
use std::fs;
const SOURCE: &str = "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";

#[test]
fn comparison_rejects_newer_drafts_review_changes_and_a_different_owner()
-> Result<(), Box<dyn std::error::Error>> {
    for decision in [ExternalDecision::KeepDrafts, ExternalDecision::UseDisk] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("fr.po");
        fs::write(&path, SOURCE)?;
        let mut catalogue = Catalogue::open(&path)?;
        let comparison = catalogue.compare()?;
        let mut other = Catalogue::open(&path)?;
        assert!(!other.comparison_is_current(&comparison.inspection));
        assert!(
            other
                .accept_external(decision, &comparison.inspection)
                .is_err()
        );
        catalogue.update(
            PoEntryId::new(0),
            Draft {
                forms: vec!["Salut.".into()],
                reviewed: true,
            },
        );
        assert!(!catalogue.comparison_is_current(&comparison.inspection));
        assert!(
            catalogue
                .accept_external(decision, &comparison.inspection)
                .is_err()
        );
        assert_eq!(
            catalogue.draft(PoEntryId::new(0)).unwrap().forms,
            ["Salut."]
        );
        let comparison = catalogue.compare()?;
        catalogue.update(
            PoEntryId::new(0),
            Draft {
                forms: vec!["Salut.".into()],
                reviewed: false,
            },
        );
        assert!(
            catalogue
                .accept_external(decision, &comparison.inspection)
                .is_err()
        );
        assert_eq!(fs::read_to_string(path)?, SOURCE);
    }
    Ok(())
}
