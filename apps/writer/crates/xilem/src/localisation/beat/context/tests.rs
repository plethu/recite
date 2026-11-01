use super::*;
use recite_writer_model::View;

fn context_fixture(source: &str) -> (tempfile::TempDir, Writer) {
    let directory = support::project(source).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer
        .follow_location(&Location {
            screen: Screen::Localise,
            document: "scene.recite".into(),
            beat: Some("start".into()),
            ..Location::default()
        })
        .unwrap();
    (directory, writer)
}

#[test]
fn incoming_prompt_pages_preserve_the_draft_and_navigate_to_the_chosen_origin() {
    let mut source = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n".to_owned();
    for index in 0..6 {
        source.push_str(&format!(
            ":: origin_{index}\n? @reply_{}\n  Question {index}?\n  -> start\n",
            index + 10
        ));
    }
    let (directory, mut writer) = context_fixture(&source);
    writer.change(|model| {
        model.select(View::Passage(FIRST.into()))?;
        model.set_draft("Pending author text.".into());
        Ok(())
    });
    let before = writer.session.model().recovery();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    let mut mounted = Mounted::new(writer, surface);
    mounted.state.navigation.sender = Some(sender);
    mounted.state.navigation.runtime = Some(runtime.handle().clone());
    assert!(!mounted.contains_text("Origin 0 → Question 0?"));
    mounted.activate("▸ Incoming prompts (6)");
    for index in 0..4 {
        assert!(mounted.contains_text(&format!("Origin {index} → Question {index}?")));
    }
    assert!(!mounted.contains_text("Origin 4 → Question 4?"));
    mounted.activate("More incoming prompts");
    assert!(!mounted.contains_text("Origin 0 → Question 0?"));
    assert!(mounted.contains_text("Origin 4 → Question 4?"));
    assert!(
        mounted
            .harness
            .get_widget_with_id(mounted.find("More incoming prompts"))
            .ctx()
            .is_disabled()
    );
    mounted.activate("▾ Incoming prompts (6)");
    assert!(!mounted.contains_text("Origin 4 → Question 4?"));
    mounted.activate("▸ Incoming prompts (6)");
    assert!(mounted.contains_text("Origin 4 → Question 4?"));
    mounted.activate("Previous incoming prompts");
    assert!(mounted.contains_text("Origin 0 → Question 0?"));
    assert_eq!(mounted.state.session.model().recovery(), before);
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
    mounted.activate("More incoming prompts");
    mounted.activate("Origin 4 → Question 4?");
    support::finish_navigation(&mut mounted.state, &mut jobs).unwrap();
    mounted.settle();
    assert_eq!(
        mounted.state.localisation.passage.as_ref().unwrap().name,
        "origin_4",
        "{}",
        mounted.state.status
    );
    assert_eq!(mounted.state.current_location().screen, Screen::Localise);
    assert!(
        mounted
            .state
            .session
            .model()
            .document()
            .source()
            .contains("Pending author text.")
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
}

#[test]
fn long_structural_context_can_be_read_in_both_directions_without_changing_source() {
    let source = format!(
        ":: start default\n:if ready()\n  !blocking overlay(\"First context {} Last context\")\n  > @ffee6p8\n    Hello.\n:else\n  > @rthh7ab\n    Otherwise spoken.\n-> END\n",
        "retained context words ".repeat(120)
    );
    let (directory, writer) = context_fixture(&source);
    let before = writer.session.model().recovery();
    let mut mounted = Mounted::new(writer, surface);
    assert!(mounted.contains_text("Within :if ready()"));
    assert!(mounted.contains_text("First context"));
    assert!(!mounted.contains_text("Last context"));
    mounted.activate("More beat context");
    assert!(!mounted.contains_text("First context"));
    assert!(mounted.contains_text("Hello."));
    mounted.activate("Previous beat context part");
    assert!(mounted.contains_text("First context"));
    for _ in 0..4 {
        mounted.activate("More beat context");
    }
    assert!(mounted.contains_text("Last context"));
    assert!(
        mounted
            .harness
            .get_widget_with_id(mounted.find("More beat context"))
            .ctx()
            .is_disabled()
    );
    assert!(mounted.contains_text("Otherwise spoken."));
    assert_eq!(mounted.state.session.model().recovery(), before);
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
}
