use recite_core::po::{PoDocument, PoSourceReplacement};

fn apply(source: &str, replacements: &[PoSourceReplacement]) -> String {
    let mut text = source.to_owned();
    for replacement in replacements.iter().rev() {
        text.replace_range(replacement.range(), replacement.text());
    }
    text
}

#[test]
fn renames_active_variant_obsolete_and_previous_contexts() -> Result<(), Box<dyn std::error::Error>>
{
    for ending in ["\n", "\r\n"] {
        let source = concat!(
            "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n",
            "msgctxt \"old_id\"\nmsgid \"Hello\"\nmsgstr \"Salut\"\n\n",
            "msgctxt \"\"\n\"old_id&formal\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\n",
            "#, fuzzy\n#| msgctxt \"old_id&casual\"\n#| msgid \"Hi\"\nmsgctxt \"old_id&casual\"\nmsgid \"Hello\"\nmsgstr \"Coucou\"\n\n",
            "#~ msgctxt \"old_id&archaic\"\n#~ msgid \"Hello\"\n#~ msgstr \"Salutations\"\n\n",
            "msgctxt \"old_id_2\"\nmsgid \"Other\"\nmsgstr \"Autre\"\n\n",
            "msgctxt \"dialogue_speaker:old_id\"\nmsgid \"Old\"\nmsgstr \"Vieux\"\n",
        )
        .replace('\n', ending);
        let document = PoDocument::parse(&source)?;
        assert!(document.has_context_key("old_id"));
        assert!(!document.has_context_key("new_id"));

        let plan = document.plan_context_rename("old_id", "new_id")?;
        assert_eq!(plan.len(), 5);
        let renamed = PoDocument::parse(apply(&source, &plan))?;
        let contexts = renamed
            .entries()
            .iter()
            .filter_map(|entry| entry.context())
            .collect::<Vec<_>>();
        assert_eq!(
            contexts,
            [
                "new_id",
                "new_id&formal",
                "new_id&casual",
                "new_id&archaic",
                "old_id_2",
                "dialogue_speaker:old_id",
            ]
        );
        assert!(renamed.source().contains(&format!(
            "#| msgctxt \"new_id&casual\"{ending}#| msgid \"Hi\""
        )));
        assert!(
            renamed
                .source()
                .contains(&format!("msgctxt \"\"{ending}\"new_id&formal\""))
        );
        assert!(!renamed.has_context_key("old_id"));
        // Untouched records keep their exact bytes.
        assert!(renamed.source().contains(&format!(
            "msgctxt \"old_id_2\"{ending}msgid \"Other\"{ending}msgstr \"Autre\""
        )));
    }
    Ok(())
}

#[test]
fn unused_ids_plan_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let document = PoDocument::parse("msgctxt \"kept\"\nmsgid \"A\"\nmsgstr \"B\"\n")?;
    assert!(document.plan_context_rename("absent", "new_id")?.is_empty());
    Ok(())
}
