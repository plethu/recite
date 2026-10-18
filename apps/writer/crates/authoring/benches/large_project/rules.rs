use super::measure;
use recite_writer_model::Document;
use serde_json::Value;

pub(super) fn run(rows: &mut Vec<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from(
        ":: start default\n? reply@22222222222222222222 requires=(standing(3))\n  Continue.\n  -> END\n",
    );
    for _ in 0..1_000 {
        source.push_str("# ");
        source.push_str(&"x".repeat(1_000));
        source.push('\n');
    }
    let document = Document::new(source)?;
    let mut rules = document.reply_rules("22222222222222222222")?;
    for _ in 0..5 {
        assert!(!measure("rules_clean_changed", rows, || rules.changed()));
        let copy = measure("rules_snapshot", rows, || rules.clone());
        assert_eq!(copy.source()?, document.source());
    }
    rules.condition = None;
    for _ in 0..5 {
        assert!(measure("rules_condition_changed", rows, || rules.changed()));
    }
    Ok(())
}
