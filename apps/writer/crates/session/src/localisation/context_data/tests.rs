use super::*;

#[test]
fn wording_variants_do_not_become_duplicate_nearby_source_passages() {
    let mut source = String::new();
    for (context, text) in [
        ("11111111111111111111", "Before"),
        ("22222222222222222222", "Current"),
        ("22222222222222222222&formal", "Current"),
        ("33333333333333333333", "After"),
        ("33333333333333333333&formal", "After"),
    ] {
        source.push_str(&format!("#. file: scene.recite\n#. block: start\nmsgctxt \"{context}\"\nmsgid \"{text}\"\nmsgstr \"\"\n\n"));
    }
    let document = PoDocument::parse(source).unwrap();
    let nearby = Nearby::new(&document);
    for context in ["22222222222222222222", "22222222222222222222&formal"] {
        let entry = document
            .entries()
            .iter()
            .find(|entry| entry.context() == Some(context))
            .unwrap();
        assert_eq!(nearby.for_entry(entry), ["Before", "After"]);
    }
}
