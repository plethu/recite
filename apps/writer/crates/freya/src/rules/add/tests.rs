use super::*;
use recite_writer_model::{Document, ProjectContext};

fn rules() -> Result<ReplyRules, Box<dyn std::error::Error>> {
    let schema = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        r#"{"schema_version":1,"conditions":{"ready":{"params":[]},"permitted":{"params":[]}},"effects":{"open":{"params":[],"modes":["immediate"]},"mark":{"params":[],"modes":["immediate"]}}}"#,
    );
    let document = Document::in_project(
        recite_core::DocumentKey::new("reply.recite")?,
        ":: start default\n? reply@22222222222222222222\n  Enter.\n  -> accepted\n:: accepted\n-> END\n",
        ProjectContext {
            schema: schema.schema,
            documents: vec![],
        },
    )?;
    Ok(document.reply_rules("22222222222222222222")?)
}

#[test]
fn adding_conditions_preserves_existing_groups_and_rejects_stale_picker_indices()
-> Result<(), Box<dyn std::error::Error>> {
    let mut rules = rules()?;
    let available = rules.available_conditions.clone();
    assert!(available.len() > 1);
    add_available_rule(&mut rules, false, None, 0);
    assert_eq!(rules.condition, Some(available[0].clone()));
    add_available_rule(&mut rules, false, None, 1);
    assert_eq!(
        rules.condition,
        Some(RuleExpression::All(vec![
            available[0].clone(),
            available[1].clone()
        ]))
    );
    add_available_rule(&mut rules, false, None, 0);
    assert_eq!(
        rules.condition,
        Some(RuleExpression::All(vec![
            available[0].clone(),
            available[1].clone(),
            available[0].clone()
        ]))
    );
    let before = rules.clone();
    add_available_rule(&mut rules, false, None, usize::MAX);
    add_available_rule(&mut rules, false, Some(&[999]), 0);
    assert_eq!(rules, before);
    Ok(())
}

#[test]
fn nested_addition_changes_only_the_selected_group_and_effect_append_keeps_order()
-> Result<(), Box<dyn std::error::Error>> {
    let mut rules = rules()?;
    let condition = rules.available_conditions[0].clone();
    rules.condition = Some(RuleExpression::All(vec![
        condition.clone(),
        RuleExpression::Any(vec![condition.clone()]),
    ]));
    add_available_rule(&mut rules, false, Some(&[1]), 0);
    assert_eq!(
        rules.condition,
        Some(RuleExpression::All(vec![
            condition.clone(),
            RuleExpression::Any(vec![condition.clone(), condition.clone()])
        ]))
    );
    let before = rules.condition.clone();
    add_available_rule(&mut rules, false, Some(&[0]), 0);
    assert_eq!(rules.condition, before, "a leaf is not an add destination");
    rules.condition = None;
    add_available_rule(&mut rules, false, Some(&[]), 0);
    assert!(
        rules.condition.is_none(),
        "a removed group must not become a root condition"
    );
    let first = rules.available_effects[0].clone();
    let second = rules.available_effects[1].clone();
    add_available_rule(&mut rules, true, None, 1);
    add_available_rule(&mut rules, true, None, 0);
    add_available_rule(&mut rules, true, None, usize::MAX);
    assert_eq!(rules.effects, vec![second, first]);
    assert!(rules.source()?.contains("22222222222222222222"));
    Ok(())
}
