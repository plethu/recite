use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Row {
    Condition(Vec<usize>, String),
    Effect(usize, String),
    Argument(ArgumentTarget),
}
pub(super) fn project(rules: &ReplyRules) -> Vec<Row> {
    let mut rows = Vec::new();
    if let Some(root) = &rules.condition {
        expression(root, Vec::new(), &mut rows);
    }
    for (index, effect) in rules.effects.iter().enumerate() {
        rows.push(Row::Effect(index, effect.function.clone()));
        arguments(CallTarget::Effect(index), effect.arguments.len(), &mut rows);
    }
    rows
}
fn expression(node: &RuleExpression, path: Vec<usize>, rows: &mut Vec<Row>) {
    if let RuleExpression::Group(inner) = node {
        let mut path = path;
        path.push(0);
        expression(inner, path, rows);
        return;
    }
    let title = match node {
        RuleExpression::Call { function, .. } => function.clone(),
        RuleExpression::All(_) => "All of these".into(),
        RuleExpression::Any(_) => "Any of these".into(),
        RuleExpression::Not(_) => "Not".into(),
        RuleExpression::Group(_) => "Group".into(),
    };
    rows.push(Row::Condition(path.clone(), title));
    match node {
        RuleExpression::Call {
            arguments: args, ..
        } => {
            arguments(CallTarget::Condition(path), args.len(), rows);
        }
        RuleExpression::All(items) | RuleExpression::Any(items) => {
            for (index, item) in items.iter().enumerate() {
                let mut path = path.clone();
                path.push(index);
                expression(item, path, rows);
            }
        }
        RuleExpression::Group(inner) | RuleExpression::Not(inner) => {
            let mut path = path;
            path.push(0);
            expression(inner, path, rows);
        }
    }
}
fn arguments(call: CallTarget, count: usize, rows: &mut Vec<Row>) {
    rows.extend((0..count).map(|index| {
        Row::Argument(ArgumentTarget {
            call: call.clone(),
            index,
        })
    }));
}
pub(super) fn same_structure(old: &ReplyRules, next: &ReplyRules) -> bool {
    project(old) == project(next)
}
