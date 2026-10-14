use super::*;

fn facts(path: &str, source: &str) -> ProjectFacts {
    let parsed = recite_parser::parse(path, source).lower_source_file();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    ProjectFacts::collect(ValidationInput::all_complete(&parsed.source_file))
}

#[test]
fn ordered_export_guard_matches_the_owned_projection() {
    let source = ":: a default\n> line@11111111111111111111\n  First.\n? reply@22222222222222222222 echo=line(11111111111111111111)\n  Reply.\n  -> a\n:: b\n-> END\n";
    let original = facts("a.recite", source);
    let mut variants = vec![original.clone()];
    for (from, to, same) in [
        ("First.", "A longer café paragraph.", true),
        ("echo=line(11111111111111111111)", "echo=none", true),
        ("  -> a", "  -> b", true),
        (":: a default", ":: renamed default", false),
        ("11111111111111111111", "33333333333333333333", false),
        (" default", "", false),
    ] {
        let next = facts("a.recite", &source.replace(from, to));
        assert_eq!(original.same_ordered_exports(&next), same, "{from}");
        variants.push(next);
    }
    let moved_default = source
        .replace(":: a default", ":: a")
        .replace(":: b", ":: b default");
    let next = facts("a.recite", &moved_default);
    assert!(original.same_ordered_exports(&next));
    variants.push(next);
    variants.push(facts("other.recite", source));

    let mut non_export_changes = original.clone();
    non_export_changes.participation = ValidationParticipation::all_incomplete();
    non_export_changes.passages[0].identity = Identity::Choice(
        Some(ChoiceId::new("11111111111111111111").unwrap()),
        ChoiceEcho::SelectedText,
    );
    non_export_changes.passages[0].frozen = !non_export_changes.passages[0].frozen;
    let mut passages = non_export_changes.passages.into_vec();
    passages.push(PassageFact {
        identity: Identity::Line(None),
        frozen: false,
        span: original.passages[0].span.clone(),
    });
    non_export_changes.passages = passages.into_boxed_slice();
    assert!(original.same_ordered_exports(&non_export_changes));
    variants.push(non_export_changes);

    for old in &variants {
        for next in &variants {
            if old.same_ordered_exports(next) {
                assert_eq!(
                    old.exports().collect::<Vec<_>>(),
                    next.exports().collect::<Vec<_>>()
                );
            }
        }
    }
}
