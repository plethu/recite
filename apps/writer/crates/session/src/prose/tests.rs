use super::{Checker, Configuration, Outcome};
use crate::ProjectSession;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const SOURCE: &str = ":: start default\n> speaker=rhea @intro\n  The lantern flickerd.\n-> END\n";
const PROSE: &str = "[prose]\nenabled = true\ndictionaries = [\"words/project.txt\"]\n";

fn project(prose: &str) -> Result<tempfile::TempDir, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        format!("format_version = 1\n[project]\ncontent_set = \"test\"\nversion = \"1\"\n{prose}"),
    )?;
    fs::write(directory.path().join("scene.recite"), SOURCE)?;
    Ok(directory)
}

fn configuration(root: &Path) -> Result<Option<Arc<Configuration>>, Box<dyn Error>> {
    Ok(Configuration::capture(&recite_config::discover_project(
        root,
    )?))
}

fn spelling(outcome: &Outcome) -> Vec<&str> {
    match outcome {
        Outcome::Findings(findings) => findings
            .iter()
            .map(|finding| finding.text.as_str())
            .collect(),
        Outcome::Setup(diagnostic) => vec![diagnostic.message.as_str()],
    }
}

#[test]
fn projects_without_prose_capture_nothing() -> Result<(), Box<dyn Error>> {
    let plain = project("")?;
    assert!(configuration(plain.path())?.is_none());
    let disabled = project("[prose]\nenabled = false\n")?;
    assert!(configuration(disabled.path())?.is_none());
    let session = ProjectSession::open(plain.path(), None)?;
    assert!(session.files().prose().is_none());
    Ok(())
}

#[test]
fn enabled_projects_check_prose_and_reuse_unchanged_findings() -> Result<(), Box<dyn Error>> {
    let directory = project(PROSE)?;
    let session = ProjectSession::open(directory.path(), None)?;
    let configuration = session.files().prose().ok_or("prose configuration")?;
    let mut checker = Checker::default();
    let source: Arc<str> = SOURCE.into();
    let first = checker.check(configuration, "scene.recite", &source);
    assert_eq!(spelling(&first), ["flickerd"]);
    let finding = first.finding(0).ok_or("finding")?;
    assert_eq!(finding.diagnostic.span.file, "scene.recite");
    assert_eq!(&SOURCE[finding.fixes[0].range.clone()], "flickerd");
    assert_eq!(first.len(), 1);
    assert!(first.diagnostic(1).is_none());
    // The same text, even in another allocation, returns the cached findings.
    let again = checker.check(configuration, "scene.recite", &Arc::from(SOURCE));
    let (Outcome::Findings(first), Outcome::Findings(again)) = (first, again) else {
        return Err("findings".into());
    };
    assert!(Arc::ptr_eq(&first, &again));
    Ok(())
}

#[test]
fn added_words_create_the_dictionary_and_suppress_the_finding() -> Result<(), Box<dyn Error>> {
    let directory = project(PROSE)?;
    let configuration = configuration(directory.path())?.ok_or("prose configuration")?;
    let mut checker = Checker::default();
    let source: Arc<str> = SOURCE.into();
    assert_eq!(
        spelling(&checker.check(&configuration, "scene.recite", &source)),
        ["flickerd"]
    );
    assert!(configuration.add_word("flickerd")?);
    assert!(!configuration.add_word("flickerd")?);
    let dictionary = directory.path().join("words/project.txt");
    assert_eq!(fs::read_to_string(dictionary)?, "flickerd\n");
    assert!(
        checker
            .check(&configuration, "scene.recite", &source)
            .is_empty()
    );
    Ok(())
}

#[test]
fn setup_problems_are_one_warning_and_have_no_dictionary() -> Result<(), Box<dyn Error>> {
    let directory = project("[prose]\nenabled = true\nlanguage = \"xx\"\n")?;
    let configuration = configuration(directory.path())?.ok_or("prose configuration")?;
    assert!(configuration.dictionary().is_none());
    assert!(configuration.add_word("flickerd").is_err());
    let outcome = Checker::default().check(&configuration, "scene.recite", &SOURCE.into());
    assert_eq!(outcome.len(), 1);
    let diagnostic = outcome.diagnostic(0).ok_or("setup warning")?;
    assert_eq!(
        diagnostic.severity,
        recite_core::DiagnosticSeverity::Warning
    );
    assert_eq!(diagnostic.span.file, "scene.recite");
    assert!(outcome.finding(0).is_none());
    Ok(())
}
