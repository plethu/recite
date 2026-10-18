#![cfg(test)]

use recite_core::{SourceRecovery, SourceRecoveryClass};
use recite_parser::{lower_source_file, parse, source_regions};

fn assert_equivalent(source: &str) {
    let whole = parse("scene.recite", source).lower_source_file();
    assert_eq!(lower_source_file("scene.recite", source), whole);
    let regions = source_regions(source);
    assert_eq!(regions.iter().map(|r| r.text()).collect::<String>(), source);
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut recovery = SourceRecovery::complete();
    for region in regions {
        assert_eq!(&source[region.byte_range()], region.text());
        let parsed = region.parse("scene.recite");
        assert_eq!(parsed.syntax().text().to_string(), region.text());
        let lowered = parsed.lower_source_file();
        assert_eq!(region.lower_source_file("scene.recite"), lowered);
        let mut roots = Vec::new();
        let report = region
            .try_visit_statements("scene.recite", |block, statement| {
                assert!(block.statements.is_empty());
                roots.push((
                    block.id.clone(),
                    block.default_speaker.clone(),
                    statement.clone(),
                ));
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        let expected = lowered
            .source_file
            .blocks
            .iter()
            .flat_map(|block| {
                block.statements.iter().map(|statement| {
                    (
                        block.id.clone(),
                        block.default_speaker.clone(),
                        statement.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(roots, expected, "streamed roots: {source:?}");
        assert_eq!(report.diagnostics, lowered.diagnostics);
        assert_eq!(report.recovery, lowered.recovery);

        blocks.extend(lowered.source_file.blocks);
        diagnostics.extend(lowered.diagnostics);
        for (complete, class) in [
            (
                lowered.recovery.ast_structure(),
                SourceRecoveryClass::AstStructure,
            ),
            (
                lowered.recovery.block_definitions(),
                SourceRecoveryClass::BlockDefinitions,
            ),
            (
                lowered.recovery.block_references(),
                SourceRecoveryClass::BlockReferences,
            ),
            (
                lowered.recovery.stable_ids(),
                SourceRecoveryClass::StableIds,
            ),
            (lowered.recovery.metadata(), SourceRecoveryClass::Metadata),
            (
                lowered.recovery.condition_functions(),
                SourceRecoveryClass::ConditionFunctions,
            ),
            (
                lowered.recovery.effect_functions(),
                SourceRecoveryClass::EffectFunctions,
            ),
            (
                lowered.recovery.inline_markup(),
                SourceRecoveryClass::InlineMarkup,
            ),
        ] {
            if !complete {
                recovery.mark(class);
            }
        }
    }
    assert_eq!(blocks, whole.source_file.blocks, "source: {source:?}");
    assert_eq!(recovery, whole.recovery, "source: {source:?}");
    // Whole-file parsing emits syntax diagnostics before lowering diagnostics;
    // authoring consumers sort them by source after composing regions.
    diagnostics.sort_by_key(|d| format!("{d:?}"));
    let mut expected = whole.diagnostics;
    expected.sort_by_key(|d| format!("{d:?}"));
    assert_eq!(diagnostics, expected, "source: {source:?}");
}

#[test]
fn restart_boundaries_preserve_recovery_and_nested_bodies() {
    let fragments = [
        "",
        "\n",
        "# preamble\n",
        "stray text\n",
        "::\n",
        ":: a default\n",
        ":: b speaker=Test\n",
        "  :: indented\n",
        "> line@11111111111111111111\n  Café 🦀\n",
        "> malformed@\n",
        "? choice\n  Choice\n  -> b\n",
        ":if ready()\n  > nested\n    Nested\n",
        ":else\n  -> END\n",
        ":match state()\n  :case yes\n    -> END\n",
        "\todd indent\n",
        "-> missing\n",
        "! effect(\n",
        "> x\n  [bad]markup\n\n",
        ":: c",
        "> no body\n",
    ];
    for left in fragments {
        for middle in fragments {
            for right in [":: end\n-> END\n", "  :: nested\n", "::\n> x\n", ""] {
                let source = format!("{left}{middle}{right}");
                assert_equivalent(&source);
                assert_equivalent(&source.replace('\n', "\r\n"));
                assert_equivalent(&source.replace('\n', "\r"));
            }
        }
    }
}

#[test]
fn all_recite_fixtures_match_whole_file_lowering() {
    fn visit(path: &std::path::Path, count: &mut usize) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, count);
            } else if path.extension().is_some_and(|ext| ext == "recite") {
                let source = std::fs::read_to_string(&path).unwrap();
                assert_equivalent(&source);
                assert_equivalent(&source.replace("\r\n", "\n").replace('\n', "\r\n"));
                assert_equivalent(&source.replace("\r\n", "\n").replace('\n', "\r"));
                *count += 1;
            }
        }
    }
    let mut count = 0;
    visit(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures"),
        &mut count,
    );
    assert!(count > 0);
}

#[test]
fn root_visitor_stops_at_the_requested_boundary() {
    let source = ":: start default\n> first\n  Hello.\n> second\n  Goodbye.\n";
    let mut visited = 0;
    let result = source_regions(source)[0].try_visit_statements("scene.recite", |_, _| {
        visited += 1;
        Err::<(), _>("cancelled")
    });
    assert_eq!(result, Err("cancelled"));
    assert_eq!(visited, 1);
}
