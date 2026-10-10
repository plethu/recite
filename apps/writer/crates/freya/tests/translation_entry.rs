mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn input(test: &TestingRunner, text: &str) -> Option<Area> {
    test.find(|node, element| {
        Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|span| span.text == text))
            .map(|_| node.layout().area)
    })
}

#[test]
fn plural_entry_keeps_variant_drafts_and_submits_all_forms()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    let source = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\n#, fuzzy\nmsgctxt \"22222222222222222222\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n\n#, fuzzy\nmsgctxt \"22222222222222222222&formal\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} titre\"\nmsgstr[1] \"{count} titres\"\n";
    std::fs::write(&path, source)?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("catalogue", &path.to_string_lossy())
        .append_pair("entry", "22222222222222222222")
        .finish();
    let route = format!("/translation?{query}");
    let mut test = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1400., 1000.),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialRoute(route));
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    let area = input(&test, "{count} billets").ok_or("second plural input")?;
    test.click_cursor((f64::from(area.max_x() - 3.), f64::from(area.center().y)));
    test.write_text(" !");
    test.sync_and_update();
    support::click(&mut test, "formal")?;
    assert!(input(&test, "{count} titres").is_some());
    support::click(&mut test, "Default wording")?;
    assert!(input(&test, "{count} billets !").is_some());
    support::click(&mut test, "Reviewed")?;
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.poll_n(std::time::Duration::from_millis(16), 8);
    let saved = recite_core::po::PoDocument::parse(std::fs::read_to_string(&path)?)?;
    let entry = saved
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("22222222222222222222"))
        .ok_or("entry")?;
    assert_eq!(entry.plural_translations()[1].text(), "{count} billets !");
    assert!(
        !entry.flags().iter().any(|f| f == "fuzzy"),
        "{}",
        std::fs::read_to_string(&path)?
    );
    if let Ok(path) = std::env::var("RECITE_WRITER_ENTRY_SCREENSHOT") {
        test.render_to_file(path);
    }
    Ok(())
}

#[test]
fn entry_context_shows_neighbours_and_notes_and_returns_from_localized_preview()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    let source = ":: start default\n> before@11111111111111111111\n  Before the translated line.\n> middle@22222222222222222222\n  Translate this line.\n> after@33333333333333333333\n  After the translated line.\n-> END\n";
    let model = recite_writer_model::Document::new(source)?;
    let template = model
        .extract_catalogue()
        .catalog
        .ok_or("template")?
        .to_pot_string();
    let document = template.replace("msgid \"Translate this line.\"\nmsgstr \"\"", "# Preserve the hesitation.\nmsgid \"Translate this line.\"\nmsgstr \"Traduire cette phrase.\"");
    let document = format!("msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n{document}");
    let catalogue = recite_core::po::PoDocument::parse(document.clone())?;
    assert!(
        catalogue
            .headers()
            .iter()
            .any(|header| header.key() == "Language" && header.value() == "fr")
    );
    std::fs::write(&path, document)?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("catalogue", &path.to_string_lossy())
        .append_pair("entry", "22222222222222222222")
        .finish();
    let route = format!("/translation?{query}");
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1400., 1200.),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialRoute(route));
            runner.provide_root_context(move || recite_writer::InitialSource(source.into()));
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    support::click(&mut test, "Nearby source · source order")?;
    for expected in [
        "Before the translated line.",
        "After the translated line.",
        "Translator notes",
        "Preserve the hesitation.",
    ] {
        assert!(
            test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(expected)))
                .is_some(),
            "{expected}"
        );
    }
    support::click(&mut test, "Nearby source · source order")?;
    support::click(&mut test, "Try this scene")?;
    support::click(&mut test, "Restart preview")?;
    support::click(&mut test, "Continue")?;
    assert!(
        test.find(
            |_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Traduire cette phrase.")
        )
        .is_some()
    );
    support::click(&mut test, "Return to translation")?;
    assert!(input(&test, "Traduire cette phrase.").is_some());
    Ok(())
}
