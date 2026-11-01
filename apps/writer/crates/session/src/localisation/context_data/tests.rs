use super::*;

#[test]
fn wording_variants_do_not_become_duplicate_nearby_source_passages() {
    let mut source = String::new();
    for (context, text) in [
        ("ffee6p8", "Before"),
        ("rthh7ab", "Current"),
        ("rthh7ab&formal", "Current"),
        ("bqyvpqq", "After"),
        ("bqyvpqq&formal", "After"),
    ] {
        source.push_str(&format!("#. file: scene.recite\n#. block: start\nmsgctxt \"{context}\"\nmsgid \"{text}\"\nmsgstr \"\"\n\n"));
    }
    let document = PoDocument::parse(source).unwrap();
    let nearby = Nearby::new(&document);
    for context in ["rthh7ab", "rthh7ab&formal"] {
        let entry = document
            .entries()
            .iter()
            .find(|entry| entry.context() == Some(context))
            .unwrap();
        assert_eq!(nearby.for_entry(entry), ["Before", "After"]);
    }
}
