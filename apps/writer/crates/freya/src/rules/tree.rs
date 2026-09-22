use recite_writer_model::RuleExpression;

pub(super) fn at<'a>(
    node: &'a mut RuleExpression,
    path: &[usize],
) -> Option<&'a mut RuleExpression> {
    let Some((first, rest)) = path.split_first() else {
        return Some(node);
    };
    match node {
        RuleExpression::All(items) | RuleExpression::Any(items) => at(items.get_mut(*first)?, rest),
        RuleExpression::Not(inner) | RuleExpression::Group(inner) if *first == 0 => at(inner, rest),
        _ => None,
    }
}

pub(super) fn remove(mut node: RuleExpression, path: &[usize]) -> Option<RuleExpression> {
    let (index, rest) = path.split_first()?;
    match &mut node {
        RuleExpression::All(items) | RuleExpression::Any(items) if *index < items.len() => {
            let child = items.remove(*index);
            if let Some(child) = remove(child, rest) {
                items.insert(*index, child);
            }
            if items.is_empty() {
                return None;
            }
        }
        RuleExpression::Group(inner) | RuleExpression::Not(inner) if *index == 0 => {
            **inner = remove(*inner.clone(), rest)?;
        }
        _ => {}
    }
    Some(node)
}
