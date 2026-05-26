use recite_runtime::ConditionQuery;

use crate::runtime_format::{RuntimeDisplayArgument, format_condition_query};

pub(super) fn condition_query_text(query: ConditionQuery<'_>) -> String {
    format_condition_query(
        query.function(),
        query
            .arguments()
            .into_iter()
            .map(RuntimeDisplayArgument::from),
    )
}
