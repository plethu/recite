mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn fill(
    test: &mut TestingRunner,
    placeholder: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text == placeholder))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("input {placeholder}"))?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.center().y)));
    test.write_text(value);
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}
#[test]
fn trial_language_count_and_fallback_are_native_controls() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"12345678901234567890\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n",
    )?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("catalogue", &path.to_string_lossy())
        .finish();
    let route = format!("/preview?{query}");
    let mut test = TestingRunner::new(recite_writer::regression_app, Size2D::new(1400., 1200.), |runner| {
        runner.provide_root_context(move || recite_writer::InitialRoute(route));
        runner.provide_root_context(|| recite_writer::InitialSource(":: start default\n> tickets@12345678901234567890 bind=(count:int=$tickets)\n  {count} ticket\n  | {count} tickets\n-> END\n".into()));
    }, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    support::click(&mut test, "Inputs for the next run")?;
    support::click(&mut test, "Choose language")?;
    fill(
        &mut test,
        "Search languages, native names or locale codes",
        "fr-CA",
    )?;
    support::click(&mut test, "French (Canada) · fr-CA")?;
    fill(&mut test, "Default wording", "formal")?;
    fill(&mut test, "tickets", "2")?;
    support::click(&mut test, "Restart preview")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "2 billets"))
            .is_some()
    );
    support::click(&mut test, "Inputs for the next run")?;
    support::click(&mut test, "What happened in this run?")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e)
            .filter(|l| l.text.contains("fr · Default wording · Translation found")))
            .is_some()
    );
    if let Ok(path) = std::env::var("RECITE_WRITER_TRIAL_SCREENSHOT") {
        test.render_to_file(path);
    }
    Ok(())
}
