use std::collections::HashMap;

use super::*;

#[test]
fn condition_answer_cache_defaults_true_and_returns_prior_answer() {
    let mut cache = HashMap::new();

    assert!(cached_condition_answer(&cache, "trusts(mira)"));
    cache.insert("trusts(mira)".to_owned(), false);

    assert!(!cached_condition_answer(&cache, "trusts(mira)"));
    assert!(cached_condition_answer(&cache, "knows(mira)"));
}
