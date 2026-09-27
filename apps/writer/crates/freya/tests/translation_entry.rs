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
