#![cfg(test)]

//! Range mapping and settings, using a scripted engine so the expectations
//! do not depend on the bundled checker's rules.

use std::ops::Range;

use recite_core::project::ProjectProse;
use recite_parser::ProseText;
use recite_prose::{
    EngineFinding, ProseChecker, ProseEngine, ProseFindingKind, ProseLanguage, ProseSettings,
    ProseSetupError, dictionary_addition,
};

/// Flags every occurrence of each needle in the checked text.
struct Scripted(Vec<&'static str>);

impl ProseEngine for Scripted {
    fn check(&mut self, text: &ProseText) -> Vec<EngineFinding> {
        let mut findings = Vec::new();
        for needle in &self.0 {
            for (start, _) in text.as_str().match_indices(needle) {
                let range = start..start + needle.len();
                findings.push(finding(range));
            }
        }
        findings
    }
}

fn finding(range: Range<usize>) -> EngineFinding {
    EngineFinding {
        kind: ProseFindingKind::Grammar,
        rule: "Scripted".to_owned(),
        message: "scripted".to_owned(),
        range: range.clone(),
        suggestions: vec![(range, "X".to_owned())],
    }
}

fn check(source: &str, needles: Vec<&'static str>) -> Vec<(String, usize)> {
    let mut checker =
        ProseChecker::with_engine(ProseSettings::default(), Box::new(Scripted(needles)));
    checker
        .check("test.recite", source)
        .into_iter()
        .map(|finding| (finding.text, finding.fixes.len()))
        .collect()
}

#[test]
fn findings_inside_one_prose_piece_get_exact_fixes() {
    let source = ":: s default\n> @a1\n  Unicode 🙂 naïve wörd here.\n";
    assert_eq!(check(source, vec!["wörd"]), [("wörd".to_owned(), 1)]);
}

#[test]
fn findings_across_line_breaks_or_markup_are_reported_without_fixes() {
    let source = ":: s default\n> @a1\n  the end of\n  the line [b]bold[/b] text\n";
    let findings = check(source, vec!["of\nthe", "line bold"]);
    assert_eq!(
        findings,
        [("of\n  the".to_owned(), 0), ("line [b]bold".to_owned(), 0)]
    );
}

#[test]
fn findings_on_placeholders_are_dropped() {
    let source = ":: s default\n> bind=(n:string=$n) @a1\n  Take a {n} home.\n";
    assert!(check(source, vec!["a {n}", "{n}"]).is_empty());
    assert_eq!(check(source, vec!["home"]), [("home".to_owned(), 1)]);
}

#[test]
fn ignored_owners_and_opt_in_metadata_are_respected() {
    let source = ":: s default\n> speaker=gob caption=\"bad\" @a1\n  bad\n? @c1\n  bad\n";
    let settings = |prose: ProjectProse| {
        ProseSettings::from_project(std::path::Path::new("/project"), &prose).expect("settings")
    };
    let run = |settings: ProseSettings| {
        ProseChecker::with_engine(settings, Box::new(Scripted(vec!["bad"])))
            .check("t.recite", source)
            .len()
    };
    assert_eq!(run(settings(ProjectProse::default())), 2);
    assert_eq!(
        run(settings(ProjectProse {
            metadata_keys: vec!["caption".to_owned()],
            ..ProjectProse::default()
        })),
        3
    );
    assert_eq!(
        run(settings(ProjectProse {
            ignore_speakers: vec!["gob".to_owned()],
            ..ProjectProse::default()
        })),
        1
    );
    assert_eq!(
        run(settings(ProjectProse {
            ignore_ids: vec!["c*".to_owned()],
            ignore_rules: vec![],
            ..ProjectProse::default()
        })),
        1
    );
    assert_eq!(
        run(settings(ProjectProse {
            ignore_rules: vec!["Scripted".to_owned()],
            ..ProjectProse::default()
        })),
        0
    );
}

#[test]
fn settings_validate_language_and_dictionary_paths() {
    let root = std::path::Path::new("/project");
    let resolve = |prose: ProjectProse| ProseSettings::from_project(root, &prose);
    assert_eq!(
        resolve(ProjectProse {
            language: Some("EN-gb".to_owned()),
            ..ProjectProse::default()
        })
        .map(|settings| settings.language()),
        Ok(ProseLanguage::EnglishGb)
    );
    assert_eq!(
        resolve(ProjectProse {
            language: Some("fr-FR".to_owned()),
            ..ProjectProse::default()
        }),
        Err(ProseSetupError::UnsupportedLanguage("fr-FR".to_owned()))
    );
    for path in ["../words.txt", "/etc/words", ""] {
        assert!(
            matches!(
                resolve(ProjectProse {
                    dictionaries: vec![path.to_owned()],
                    ..ProjectProse::default()
                }),
                Err(ProseSetupError::Dictionary { .. })
            ),
            "{path:?} must be rejected"
        );
    }
    let settings = resolve(ProjectProse {
        dictionaries: vec!["names.txt".to_owned(), "lore/terms.txt".to_owned()],
        ..ProjectProse::default()
    })
    .expect("settings");
    assert_eq!(
        settings.dictionary_for_additions(),
        Some(root.join("names.txt").as_path())
    );
    let diagnostic = ProseSetupError::UnsupportedLanguage("fr-FR".to_owned())
        .diagnostic("recite.project.toml")
        .expect("diagnostic");
    assert_eq!(diagnostic.code.as_str(), "RECITE_PROSE101");
}

#[test]
fn dictionary_additions_append_one_line() {
    let edit = dictionary_addition(None, "Vexmoor").expect("edit");
    assert_eq!((edit.range, edit.replacement.as_str()), (0..0, "Vexmoor\n"));
    let edit = dictionary_addition(Some("# names\nRhea"), "Zoë").expect("edit");
    assert_eq!((edit.range, edit.replacement.as_str()), (12..12, "\nZoë\n"));
    assert!(dictionary_addition(Some("Rhea\n"), "Rhea").is_none());
    assert!(dictionary_addition(Some(""), "two\nlines").is_none());
    assert!(dictionary_addition(Some(""), "# comment").is_none());
}
