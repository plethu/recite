mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;

fn open(source: &str) -> Result<(tempfile::TempDir, TestingRunner), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    std::fs::write(dir.path().join("scene.recite"), source)?;
    let path = dir.path().to_owned();
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        (1200., 800.).into(),
        move |runner| {
            runner.provide_root_context(move || recite_writer::InitialProject(Some(path)));
            runner.provide_root_context(|| {
                recite_writer::InitialRoute(
                    "recite://writer/write?scene=scene.recite&view=source".into(),
                )
            });
        },
        1.,
    )
    .0;
    wait(&mut test, |test| {
        has(test, "scene.recite")
            && source
                .lines()
                .filter(|line| !line.is_empty())
                .all(|line| has(test, line))
    })?;
    // The initial route applies after the requested Source tree is mounted.
    // Drain those UI effects before issuing a new view intent.
    test.poll_n(Duration::from_millis(16), 3);
    Ok((dir, test))
}
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Label::try_downcast(e)
            .is_some_and(|l| l.text.contains(text))
            .then_some(())
            .or_else(|| {
                Paragraph::try_downcast(e)
                    .is_some_and(|p| {
                        p.spans
                            .iter()
                            .map(|s| s.text.as_ref())
                            .collect::<String>()
                            .contains(text)
                    })
                    .then_some(())
            })
    })
    .is_some()
}
fn wait(
    test: &mut TestingRunner,
    done: impl Fn(&TestingRunner) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..10_000 {
        test.sync_and_update();
        if done(test) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let texts = test.find_many(|_, e| {
        Label::try_downcast(e)
            .map(|l| l.text.to_string())
            .or_else(|| {
                Paragraph::try_downcast(e)
                    .map(|p| p.spans.iter().map(|s| s.text.as_ref()).collect::<String>())
            })
    });
    Err(format!("cold view did not settle: {texts:?}").into())
}
fn action(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (number, code) = match name {
        "Script" => ("1", Code::Digit1),
        "Map" => ("2", Code::Digit2),
        "Source" => ("3", Code::Digit3),
        _ => return Err("unknown view".into()),
    };
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character(number.into()),
        code,
        modifiers: Modifiers::CONTROL,
    });
    Ok(())
}
const SOURCE: &str =
    ":: start default\n> line@11111111111111111111\n  Hello from the cold source.\n-> END\n";
#[test]
fn cold_source_to_script_and_map_preserve_content_and_navigation()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut test) = open(SOURCE)?;
    action(&mut test, "Script")?;
    wait(&mut test, |test| has(test, "Add line"))?;
    assert!(has(&test, "Hello from the cold source."));
    support::click(&mut test, "Source")?;
    assert!(has(&test, "> line@11111111111111111111"));
    support::click(&mut test, "Map")?;
    assert!(has(&test, "Start"));
    Ok(())
}
#[test]
fn cancelling_a_deferred_view_keeps_source_after_preparation()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut test) = open(SOURCE)?;
    // Both accepted commands run before the root result future is polled.
    action(&mut test, "Script")?;
    action(&mut test, "Source")?;
    test.poll_n(Duration::from_millis(16), 30);
    assert!(has(&test, "> line@11111111111111111111"));
    assert!(!has(&test, "Add line"));
    action(&mut test, "Script")?;
    wait(&mut test, |test| has(test, "Add line"))?;
    Ok(())
}
#[test]
fn repeated_invalid_source_view_commands_keep_the_error_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut test) = open(":: broken\n? invalid\n")?;
    for _ in 0..2 {
        action(&mut test, "Script")?;
        wait(&mut test, |test| has(test, "has no unique frozen ID"))?;
        assert!(has(&test, ":: broken"));
    }
    Ok(())
}
