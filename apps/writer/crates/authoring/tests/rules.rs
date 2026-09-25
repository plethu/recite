use recite_writer_model::{Document, RuleExpression, View, Workbench};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n? reply@22222222222222222222 requires=(standing(3) and (has_key(gate) or has_permit(courier)))\n  Let me through.\n  -> accepted\n:: accepted\n! immediate play_sfx(gate_latch)\n# Keep this comment.\n! blocking mark_map(floodgate)\n-> END\n";

#[test]
fn unchanged_rules_are_byte_identical_and_edits_are_one_undo() -> TestResult {
    let mut model = Workbench::new(SOURCE)?;
    let mut rules = model.document().reply_rules("22222222222222222222")?;
    assert_eq!(rules.source()?, SOURCE);
    let Some(RuleExpression::Group(inner)) = &mut rules.condition else {
        return Err("outer group".into());
    };
    let RuleExpression::All(items) = inner.as_mut() else {
        return Err("all group".into());
    };
    let RuleExpression::Call { arguments, .. } = &mut items[0] else {
        return Err("condition call".into());
    };
    arguments[0].value = "5".into();
    rules.effects[0].arguments[0].value = "bell".into();
    let edited = rules.source()?;
    assert!(edited.contains("standing(5)"));
    assert!(edited.contains("! immediate play_sfx(bell)\n# Keep this comment.\n"));
    assert!(edited.contains("reply@22222222222222222222"));
    model.select(View::Source)?;
    model.set_draft(edited.clone());
    model.apply_reply_rules(&rules)?;
    assert_eq!(model.document().source(), edited);
    model.undo()?;
    assert_eq!(model.document().source(), SOURCE);
    Ok(())
}

#[test]
fn invalid_argument_draft_is_preserved_and_not_applied() -> TestResult {
    let mut model = Workbench::new(SOURCE)?;
    let mut rules = model.document().reply_rules("22222222222222222222")?;
    let Some(RuleExpression::Group(inner)) = &mut rules.condition else {
        return Err("outer group".into());
    };
    let RuleExpression::All(items) = inner.as_mut() else {
        return Err("all group".into());
    };
    let RuleExpression::Call { arguments, .. } = &mut items[0] else {
        return Err("condition call".into());
    };
    arguments[0].value = "-".into();
    model.select(View::Source)?;
    model.set_draft(rules.source()?);
    assert!(model.apply_reply_rules(&rules).is_err());
    assert!(model.draft().contains("standing(-)"));
    assert_eq!(model.document().source(), SOURCE);
    Ok(())
}

#[test]
fn stale_rules_do_not_replace_new_source() -> TestResult {
    let mut doc = Document::new(SOURCE)?;
    let rules = doc.reply_rules("22222222222222222222")?;
    doc.replace_source(doc.revision(), SOURCE.replace("Hello.", "Hello again."))?;
    assert!(rules.validate(&doc).is_err());
    Ok(())
}

#[test]
fn effects_reorder_without_moving_comments_or_changing_ids() -> TestResult {
    let model = Workbench::new(SOURCE)?;
    let mut rules = model.document().reply_rules("22222222222222222222")?;
    assert!(rules.effect_order_editable);
    rules.effects.swap(0, 1);
    let source = rules.source()?;
    assert!(source.contains(
        "! blocking mark_map(floodgate)\n# Keep this comment.\n! immediate play_sfx(gate_latch)"
    ));
    rules.validate(model.document())?;
    Ok(())
}

#[test]
fn unicode_string_edits_preserve_crlf_and_escape_literals() -> TestResult {
    let source = SOURCE
        .replace("play_sfx(gate_latch)", "play_sfx(\"cloch 🔔\")")
        .replace('\n', "\r\n");
    let document = Document::new(source.clone())?;
    let mut rules = document.reply_rules("22222222222222222222")?;
    rules.effects[0].arguments[0].value = "\"ding\"\n\\dong".into();
    let edited = rules.validate(&document)?;
    assert!(edited.contains("play_sfx(\"\\\"ding\\\"\\n\\\\dong\")\r\n"));
    assert_eq!(
        edited.matches("\r\n").count(),
        source.matches("\r\n").count()
    );
    let reloaded = Document::new(edited)?.reply_rules("22222222222222222222")?;
    assert_eq!(reloaded.effects[0].arguments[0].value, "\"ding\"\n\\dong");
    Ok(())
}

#[test]
fn adding_an_existing_error_cannot_hide_behind_its_original_diagnostic() -> TestResult {
    let document = Document::in_project(
        recite_core::DocumentKey::new("test.recite")?,
        SOURCE,
        recite_writer_model::ProjectContext {
            schema: Some(recite_core::schema::ProjectSchema::empty_v1()),
            documents: vec![],
        },
    )?;
    let mut rules = document.reply_rules("22222222222222222222")?;
    rules.effects.push(rules.effects[0].clone());
    assert!(rules.validate(&document).is_err());
    Ok(())
}

#[test]
fn other_branch_statements_prevent_effect_reordering() -> TestResult {
    let source = SOURCE.replace(
        "# Keep this comment.",
        "> nested@33333333333333333333\n  Wait.",
    );
    let document = Document::new(source.clone())?;
    let mut rules = document.reply_rules("22222222222222222222")?;
    assert!(!rules.effect_order_editable);
    assert_eq!(rules.source()?, source);
    rules.effects.swap(0, 1);
    assert!(rules.source().is_err());
    Ok(())
}

#[test]
fn schema_choices_additions_and_delivery_are_validated() -> TestResult {
    use recite_core::{
        DocumentKey, ast::EffectMode, schema::ConditionDefinition, schema::ConditionReturnType,
        schema::EffectDefinition, schema::ParameterDefinition, schema::ProjectSchema,
        schema::RegistryDefinition, schema::SchemaTypeRef,
    };
    let mut schema = ProjectSchema::empty_v1();
    schema.registries.insert(
        "places".into(),
        RegistryDefinition {
            values: ["gate".into(), "station".into()].into(),
            ..RegistryDefinition::default()
        },
    );
    let parameter = ParameterDefinition {
        name: "place".into(),
        type_ref: SchemaTypeRef::Registry("places".into()),
    };
    schema.conditions.insert(
        "visited".into(),
        ConditionDefinition {
            params: vec![parameter.clone()],
            returns: ConditionReturnType::Bool,
            availability_reason: None,
        },
    );
    schema.effects.insert(
        "visit".into(),
        EffectDefinition {
            params: vec![parameter],
            modes: [EffectMode::Blocking].into(),
        },
    );
    let source = ":: start default\n> line@11111111111111111111\n  Hello.\n? reply@22222222222222222222\n  Go.\n  -> accepted\n:: accepted\n-> END\n";
    let document = Document::in_project(
        DocumentKey::new("test.recite")?,
        source,
        recite_writer_model::ProjectContext {
            schema: Some(schema),
            documents: vec![],
        },
    )?;
    let mut rules = document.reply_rules("22222222222222222222")?;
    rules.condition = Some(rules.available_conditions[0].clone());
    rules.effects.push(rules.available_effects[0].clone());
    let edited = rules.validate(&document)?;
    assert!(edited.contains("requires=(visited(gate))"));
    assert!(edited.contains("! blocking visit(gate)\n-> END"));
    assert_eq!(rules.effects[0].arguments[0].choices, ["gate", "station"]);
    rules.effects[0].arguments[0].value = "unknown".into();
    assert!(rules.validate(&document).is_err());
    rules.effects[0].arguments[0].value = "station".into();
    rules.effects[0].mode = EffectMode::Immediate;
    assert!(rules.validate(&document).is_err());
    rules.effects[0].mode = EffectMode::Blocking;
    rules.validate(&document)?;
    rules.condition = None;
    rules.effects.clear();
    assert_eq!(rules.source()?, source);
    Ok(())
}
