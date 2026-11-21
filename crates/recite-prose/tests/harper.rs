#![cfg(test)]
#![cfg(feature = "harper")]

mod support;

use std::sync::{Mutex, OnceLock};

use recite_parser::{ProseRegionKind, prose_regions};
use recite_prose::{ProseChecker, ProseFinding, ProseFindingKind};

use support::{assert_snapshot, fixture, render, settings};

const HARBOUR: &str = "fixtures/recite/valid/prose/harbour.recite";
const INCOMPLETE: &str = "fixtures/recite/invalid/prose/incomplete.recite";

/// The project configuration used by the proof: a dictionary for invented
/// names and opted-in `caption` metadata. Building Harper is slow in debug
/// builds, so tests share one checker.
fn project_checker() -> &'static Mutex<ProseChecker> {
    static CHECKER: OnceLock<Mutex<ProseChecker>> = OnceLock::new();
    CHECKER.get_or_init(|| {
        let settings = settings(|prose| {
            prose.language = Some("en-GB".to_owned());
            prose.dictionaries = vec!["words.txt".to_owned()];
            prose.metadata_keys = vec!["caption".to_owned()];
        });
        Mutex::new(ProseChecker::new(settings).expect("checker"))
    })
}

fn check(relative: &str) -> (String, Vec<ProseFinding>) {
    let source = fixture(relative);
    let findings = project_checker()
        .lock()
        .expect("checker lock")
        .check(relative, &source);
    (source, findings)
}

#[test]
fn harbour_findings_match_snapshot() {
    let (source, findings) = check(HARBOUR);
    assert_snapshot("harbour_findings", &render(&source, &findings));
}

#[test]
fn incomplete_file_is_checked_while_editing() {
    let (source, findings) = check(INCOMPLETE);
    assert_snapshot("incomplete_findings", &render(&source, &findings));
    let texts = findings
        .iter()
        .map(|finding| finding.text.as_str())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"flickerd"), "{texts:?}");
    assert!(texts.contains(&"choise"), "{texts:?}");
}

#[test]
fn proof_detects_spelling_and_grammar_in_prose() {
    let (_, findings) = check(HARBOUR);
    let spelling = findings
        .iter()
        .find(|finding| finding.text == "recieved")
        .expect("spelling error in a line body");
    assert_eq!(spelling.kind, ProseFindingKind::Spelling);
    assert_eq!(spelling.diagnostic.code.as_str(), "RECITE_PROSE001");
    let grammar = findings
        .iter()
        .find(|finding| finding.kind == ProseFindingKind::Grammar && finding.text.contains("an"))
        .expect("article error in a nested choice");
    assert_eq!(grammar.region, ProseRegionKind::Choice);
    assert_eq!(grammar.stable_id.as_deref(), Some("go_market"));
    assert_eq!(grammar.diagnostic.code.as_str(), "RECITE_PROSE002");
    assert!(
        findings
            .iter()
            .any(|finding| finding.text == "Teh" && finding.region == ProseRegionKind::Metadata),
        "opted-in caption metadata is checked"
    );
}

#[test]
fn findings_never_cover_recite_syntax() {
    let (source, findings) = check(HARBOUR);
    let regions = prose_regions(&source);
    for finding in &findings {
        assert!(
            regions
                .iter()
                .any(|region| region.bytes().start <= finding.range.start
                    && finding.range.end <= region.bytes().end),
            "finding {:?} is outside prose",
            finding.text
        );
        for syntax in ["{", "}", "[slow]", "[/slow]", "@", "speaker=", "\\"] {
            assert!(
                !finding.text.contains(syntax),
                "{syntax} in {:?}",
                finding.text
            );
        }
    }
    let flagged = findings
        .iter()
        .flat_map(|finding| finding.text.split_whitespace())
        .collect::<Vec<_>>();
    for clean in [
        "Vexmoor",
        "count",
        "name",
        "item",
        "a",
        "slow",
        "rhea",
        "hazel",
        "harbour_arrival",
        "café",
        "naïve",
        "Zoë",
        "recieve",
        "teh",
        "speling",
        "baker_open",
        "🙂",
    ] {
        assert!(
            !flagged.contains(&clean),
            "{clean} must not be flagged: {flagged:?}"
        );
    }
}

#[test]
fn every_fix_edits_prose_only_and_keeps_structure() {
    let (source, findings) = check(HARBOUR);
    let original = prose_regions(&source);
    let mut applied = 0;
    for finding in &findings {
        for fix in &finding.fixes {
            let mut edited = source.clone();
            edited.replace_range(fix.range.clone(), &fix.replacement);
            let regions = prose_regions(&edited);
            assert_eq!(
                regions.len(),
                original.len(),
                "fix {fix:?} changed structure"
            );
            for (before, after) in original.iter().zip(&regions) {
                assert_eq!(before.kind(), after.kind());
                assert_eq!(before.stable_id(), after.stable_id());
                assert_eq!(before.speaker(), after.speaker());
            }
            let outside = |text: &str, from: usize| {
                let (left, right) = text.split_at(from);
                (left.to_owned(), right.to_owned())
            };
            let (before_left, _) = outside(&source, fix.range.start);
            let (edited_left, _) = outside(&edited, fix.range.start);
            assert_eq!(before_left, edited_left);
            assert_eq!(
                &source[fix.range.end..],
                &edited[fix.range.start + fix.replacement.len()..]
            );
            applied += 1;
        }
    }
    assert!(applied > 0, "the proof applies at least one fix");
    let spelling = findings
        .iter()
        .find(|finding| finding.text == "recieved")
        .expect("spelling finding");
    let fix = &spelling.fixes[0];
    let mut edited = source.clone();
    edited.replace_range(fix.range.clone(), &fix.replacement);
    assert!(edited.contains("I received your letter this morning."));
    let rechecked = project_checker()
        .lock()
        .expect("checker lock")
        .check(HARBOUR, &edited);
    assert!(rechecked.iter().all(|finding| finding.text != "recieved"));
}

#[test]
fn project_dictionary_and_suppressions_apply() {
    let source = fixture(HARBOUR);
    let mut bare = ProseChecker::new(settings(|_| {})).expect("checker");
    let findings = bare.check(HARBOUR, &source);
    assert!(findings.iter().any(|finding| finding.text == "Vexmoor"));
    assert!(
        findings
            .iter()
            .all(|finding| finding.region != ProseRegionKind::Metadata),
        "metadata is opt-in"
    );

    let mut suppressed = ProseChecker::new(settings(|prose| {
        prose.dictionaries = vec!["words.txt".to_owned()];
        prose.ignore_rules = vec!["SpellCheck".to_owned()];
        prose.ignore_speakers = vec!["rhea".to_owned()];
        prose.ignore_ids = vec!["go_*".to_owned()];
    }))
    .expect("checker");
    let findings = suppressed.check(HARBOUR, &source);
    assert!(findings.iter().all(|finding| finding.rule != "SpellCheck"));
    assert!(
        findings
            .iter()
            .all(|finding| finding.stable_id.as_deref() != Some("go_market")
                && finding.stable_id.as_deref() != Some("rhea_arrival"))
    );
}
