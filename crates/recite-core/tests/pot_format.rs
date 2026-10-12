#![cfg(test)]

use recite_core::po::{PoDocument, PotDocument, PotEntry, PotReference};

#[test]
fn pot_preserves_unicode_control_escapes_plural_forms_and_reference_bounds() {
    let text = "Café 张伟 \"quote\" \\ \n\r\t\u{7}\u{8}\u{c}\u{b}\0\u{1b}\u{7f}\u{85}";
    let document = PotDocument {
        entries: vec![
            PotEntry {
                context: "11111111111111111111".to_owned(),
                source_text: text.to_owned(),
                plural_source_text: None,
                comments: vec!["Café\n张伟\tcontrol\u{7f}".to_owned()],
                reference: Some(PotReference {
                    file: "dialogue/张伟:name\npart\t\u{7f}.recite".to_owned(),
                    line: 0,
                    column: u32::MAX,
                }),
            },
            PotEntry {
                context: "22222222222222222222".to_owned(),
                source_text: "One \"café\" {count}".to_owned(),
                plural_source_text: Some("Many \\ cafés\n{count}".to_owned()),
                comments: Vec::new(),
                reference: Some(PotReference {
                    file: "dialogue/plural.recite".to_owned(),
                    line: u32::MAX,
                    column: 0,
                }),
            },
        ],
    };
    let output = document.to_pot_string();
    assert_eq!(
        output,
        concat!(
            "#. Café 张伟 control \n",
            "#: dialogue/张伟_name_part__.recite:0:4294967295\n",
            "msgctxt \"11111111111111111111\"\n",
            "msgid \"Café 张伟 \\\"quote\\\" \\\\ \\n\\r\\t\\a\\b\\f\\v\\000\\033\\177\\205\"\n",
            "msgstr \"\"\n\n",
            "#: dialogue/plural.recite:4294967295:0\n",
            "msgctxt \"22222222222222222222\"\n",
            "msgid \"One \\\"café\\\" {count}\"\n",
            "msgid_plural \"Many \\\\ cafés\\n{count}\"\n",
            "msgstr[0] \"\"\nmsgstr[1] \"\"\n",
        )
    );
    let parsed = PoDocument::parse(output).expect("locale-neutral POT reparses");
    assert_eq!(parsed.entries()[0].source_text(), text);
    assert_eq!(parsed.entries()[1].source_text(), "One \"café\" {count}");
    assert_eq!(
        parsed.entries()[1].plural_source_text(),
        Some("Many \\ cafés\n{count}")
    );
}

#[test]
fn pot_escapes_context_and_retains_empty_fields() {
    let document = PotDocument {
        entries: vec![PotEntry {
            context: "Élodie \"quoted\"\\\n".to_owned(),
            source_text: String::new(),
            plural_source_text: Some(String::new()),
            comments: Vec::new(),
            reference: None,
        }],
    };
    assert_eq!(
        document.to_pot_string(),
        concat!(
            "msgctxt \"Élodie \\\"quoted\\\"\\\\\\n\"\n",
            "msgid \"\"\nmsgid_plural \"\"\n",
            "msgstr[0] \"\"\nmsgstr[1] \"\"\n",
        )
    );
    assert_eq!(
        PotDocument {
            entries: Vec::new()
        }
        .to_pot_string(),
        ""
    );
}
