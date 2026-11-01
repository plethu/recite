use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{
    EditOperation, RuleArgument, RuleExpression, SourceValidation, Workbench,
};

const SOURCE: &str = "# preserve this comment\r\n:: start default\r\n? requires=(standing(3)) @rthh7ab\r\n  Continue.\r\n  # preserve divert\r\n  -> END\r\n";

fn first_argument(node: &mut RuleExpression) -> Option<&mut RuleArgument> {
    match node {
        RuleExpression::Call { arguments, .. } => arguments.first_mut(),
        RuleExpression::All(items) | RuleExpression::Any(items) => {
            items.iter_mut().find_map(first_argument)
        }
        RuleExpression::Not(inner) | RuleExpression::Group(inner) => first_argument(inner),
    }
}

#[test]
fn staged_invalid_arguments_are_recoverable_and_do_not_change_accepted_source() {
    let mut model = Workbench::new(SOURCE).unwrap();
    let mut rules = model.open_reply_rules("rthh7ab").unwrap();
    model
        .stage_reply_rules(&mut rules, |rules| {
            first_argument(rules.condition.as_mut().unwrap())
                .unwrap()
                .value = "-".into();
        })
        .unwrap();
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
    assert!(model.draft().contains("standing(-)"));
    assert!(model.draft().starts_with("# preserve this comment\r\n"));
    assert!(model.draft().contains("# preserve divert\r\n  -> END\r\n"));
    assert_eq!(model.document().source(), SOURCE);
    assert!(
        model
            .edit_request(EditOperation::Apply)
            .unwrap()
            .unwrap()
            .with_rules(rules.clone())
            .prepare(&CancellationToken::new())
            .is_err()
    );
    let recovery = model.recovery();
    let mut restored = Workbench::new(SOURCE).unwrap();
    recovery.restore(&mut restored).unwrap();
    assert_eq!(restored.draft(), rules.source().unwrap());
    assert!(restored.apply().is_err());
}

#[test]
fn stale_source_draft_rejects_rule_staging_without_mutating_either_owner() {
    let mut model = Workbench::new(SOURCE).unwrap();
    let mut rules = model.open_reply_rules("rthh7ab").unwrap();
    model.set_draft(SOURCE.replace("Continue.", "Independent source edit."));
    let before = model.recovery();
    let previous_rules = rules.clone();
    assert!(
        model
            .stage_reply_rules(&mut rules, |rules| rules.condition = None)
            .is_err()
    );
    assert_eq!(model.recovery(), before);
    assert_eq!(rules, previous_rules);
}
