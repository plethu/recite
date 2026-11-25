//! A match nested in another match's arm compiles to its own arm range.

use recite_core::compiled::{CompiledMatchPattern, MatchArmRange};

use super::*;

fn arm_patterns(
    dialogue: &recite_core::compiled::CompiledDialogue,
    range: MatchArmRange,
) -> Vec<String> {
    let start = range.start.as_u32() as usize;
    dialogue.match_arms[start..start + range.len as usize]
        .iter()
        .map(|arm| match &arm.pattern {
            CompiledMatchPattern::Variant(variant) => variant.clone(),
            CompiledMatchPattern::Wildcard => "_".to_owned(),
        })
        .collect()
}

#[test]
fn nested_matches_own_contiguous_arm_ranges_in_source_order() {
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue/main.recite",
            concat!(
                ":: start default\n",
                ":match outer(thread)\n",
                "  :case tired\n",
                "    :match middle(thread)\n",
                "      :case calm\n",
                "        :match inner(thread)\n",
                "          :case near\n",
                "            -> END\n",
                "          :case _\n",
                "            -> END\n",
                "      :case _\n",
                "        -> END\n",
                "  :case rested\n",
                "    -> END\n",
                "  :case _\n",
                "    -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("source map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let dialogue = report.asset.expect("asset").dialogue;
    let matches = dialogue
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            CompiledStatementKind::Match { arms, .. } => Some(*arms),
            _ => None,
        })
        .map(|arms| arm_patterns(&dialogue, arms))
        .collect::<Vec<_>>();
    let mut patterns = matches.clone();
    patterns.sort();
    assert_eq!(
        patterns,
        [
            vec!["calm".to_owned(), "_".to_owned()],
            vec!["near".to_owned(), "_".to_owned()],
            vec!["tired".to_owned(), "rested".to_owned(), "_".to_owned()],
        ]
    );
    // Every compiled arm belongs to exactly one match.
    assert_eq!(
        matches.iter().map(Vec::len).sum::<usize>(),
        dialogue.match_arms.len()
    );
}
