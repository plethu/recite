use super::*;
const SOURCE: &str = ":: start default\n? reply@22222222222222222222 requires=(standing(3))\n  Continue.\n  -> accepted\n:: accepted\n! immediate play_sfx(gate_latch)\n! blocking mark_map(floodgate)\n-> END\n";

#[test]
fn cheap_dirty_checks_match_the_original_checked_source_comparison() {
    let document = Document::new(SOURCE).unwrap();
    let original = document.reply_rules("22222222222222222222").unwrap();
    let edits: [fn(&mut ReplyRules); 8] = [
        |_| {},
        |r| r.condition = None,
        |r| r.effects[0].arguments[0].value = "changed".into(),
        |r| r.effects.swap(0, 1),
        |r| {
            r.effects.pop();
        },
        |r| r.effects.push(r.effects[0].clone()),
        |r| r.effects[0].mode = recite_core::ast::EffectMode::Deferred,
        |r| r.effects[0].function = "ignored for original effect".into(),
    ];
    for edit in edits {
        let mut rules = original.clone();
        edit(&mut rules);
        let expected = rules
            .source()
            .is_ok_and(|source| source.as_str() != rules.source.as_ref())
            || rules.condition != rules.original_condition
            || rules
                .effects
                .iter()
                .any(|e| e.arguments.iter().any(RuleArgument::changed));
        assert_eq!(rules.changed(), expected);
        assert!(std::sync::Arc::ptr_eq(&original.source, &rules.source));
    }
}
