use super::*;
#[cfg(target_os = "linux")]
use freya_testing::prelude::*;

#[test]
fn inconsistent_beat_and_passage_link_does_not_change_selection() -> Result<(), WorkbenchError> {
    let mut model = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let before = model.view().clone();
    let passage = model.document().passage_snapshot()?[0].id.clone();
    let location = Location {
        beat: Some("not_the_passage_beat".into()),
        passage: Some(passage),
        ..Location::default()
    };
    assert!(select(&mut model, &location).is_err());
    assert_eq!(model.view(), &before);
    Ok(())
}

#[test]
fn linked_catalogue_must_stay_inside_its_project() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("locale"))?;
    let inside = root.path().join("locale/fr.po");
    std::fs::write(&inside, "")?;
    std::fs::write(outside.path().join("outside.po"), "")?;
    assert_eq!(catalogue_path(root.path(), "locale/fr.po")?, inside);
    assert!(catalogue_path(root.path(), "../outside.po").is_err());
    assert!(catalogue_path(root.path(), outside.path().to_str().ok_or("path")?).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            outside.path().join("outside.po"),
            root.path().join("locale/link.po"),
        )?;
        assert!(catalogue_path(root.path(), "locale/link.po").is_err());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linked_project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    std::fs::write(dir.path().join("a.recite"), recite_writer_model::FIXTURE)?;
    std::fs::write(
        dir.path().join("b.recite"),
        ":: other default\n> other@33333333333333333333\n  Other scene.\n-> END\n",
    )?;
    Ok(dir)
}

#[cfg(target_os = "linux")]
fn submitted_route(
    sender: &std::sync::mpsc::SyncSender<crate::activation::IncomingRoute>,
    route: String,
) -> Result<std::sync::mpsc::Receiver<Result<(), String>>, Box<dyn std::error::Error>> {
    let (reply, result) = std::sync::mpsc::channel();
    sender.try_send(crate::activation::IncomingRoute {
        project: project_from_route(&route)?,
        route: Some(route),
        cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        reply,
    })?;
    Ok(result)
}

#[cfg(target_os = "linux")]
#[test]
fn welcome_owner_opens_forwarded_projects_and_switches_cleanly()
-> Result<(), Box<dyn std::error::Error>> {
    let first = linked_project()?;
    let second = linked_project()?;
    std::fs::write(
        second.path().join("b.recite"),
        ":: other default\n> other@33333333333333333333\n  Second project scene.\n-> END\n",
    )?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    let inbox = crate::ActivationInbox::new(receiver);
    let (mut test, _) = TestingRunner::new(
        crate::editor_app,
        Size2D::new(1400., 1000.),
        move |runner| {
            runner.provide_root_context(move || crate::InitialProject(None));
            runner.provide_root_context(move || inbox);
        },
        1.,
    );
    test.poll_n(std::time::Duration::from_millis(16), 15);
    let route = |root: &std::path::Path| {
        format!(
            "recite://writer/write?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("project", &root.to_string_lossy())
                .append_pair("scene", "b.recite")
                .append_pair("beat", "other")
                .finish()
        )
    };
    let (cancel_reply, cancel_result) = std::sync::mpsc::channel();
    sender.try_send(crate::activation::IncomingRoute {
        project: Some(first.path().to_path_buf()),
        route: Some(route(first.path())),
        cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        reply: cancel_reply,
    })?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert!(cancel_result.try_recv()?.is_err());
    let first_reply = submitted_route(&sender, route(first.path()))?;
    test.poll_n(std::time::Duration::from_millis(16), 40);
    assert_eq!(first_reply.try_recv()?, Ok(()));
    assert!(contains_paragraph(&test, "Other scene."));
    let second_reply = submitted_route(&sender, route(second.path()))?;
    test.poll_n(std::time::Duration::from_millis(16), 40);
    assert_eq!(second_reply.try_recv()?, Ok(()));
    assert!(contains_paragraph(&test, "Second project scene."));
    let third = linked_project()?;
    let missing_scene = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &third.path().to_string_lossy())
            .append_pair("scene", "missing.recite")
            .finish()
    );
    let failed_route = submitted_route(&sender, missing_scene)?;
    test.poll_n(std::time::Duration::from_millis(16), 40);
    assert!(failed_route.try_recv()?.unwrap_err().contains("Project "));
    let third_reply = submitted_route(&sender, route(third.path()))?;
    test.poll_n(std::time::Duration::from_millis(16), 20);
    assert_eq!(third_reply.try_recv()?, Ok(()));
    assert!(contains_paragraph(&test, "Other scene."));
    Ok(())
}

#[cfg(target_os = "linux")]
fn contains_paragraph(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, element| {
        Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|span| span.text.contains(text)))
    })
    .is_some()
}

#[cfg(target_os = "linux")]
#[test]
fn received_route_updates_history_and_refuses_a_source_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = linked_project()?;
    let root = dir.path().to_path_buf();
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    let inbox = crate::ActivationInbox::new(receiver);
    let initial = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &root.to_string_lossy())
            .append_pair("scene", "a.recite")
            .finish()
    );
    let target = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &root.to_string_lossy())
            .append_pair("scene", "b.recite")
            .append_pair("beat", "other")
            .finish()
    );
    let (mut test, _) = TestingRunner::new(
        crate::editor_app,
        Size2D::new(1400., 1000.),
        move |runner| {
            runner.provide_root_context(move || crate::InitialProject(Some(root)));
            runner.provide_root_context(move || InitialRoute(initial));
            runner.provide_root_context(move || inbox);
        },
        1.,
    );
    test.poll_n(std::time::Duration::from_millis(16), 40);
    let received = submitted_route(&sender, target.clone())?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert_eq!(received.try_recv()?, Ok(()));
    assert!(contains_paragraph(&test, "Other scene."));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::ArrowLeft),
        code: Code::ArrowLeft,
        modifiers: Modifiers::ALT,
    });
    test.poll_n(std::time::Duration::from_millis(16), 8);
    assert!(!contains_paragraph(&test, "Other scene."));
    let source = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == "Source")
                .map(|_| node.layout().area)
        })
        .ok_or("Source")?;
    test.click_cursor((f64::from(source.center().x), f64::from(source.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 5);
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("which_way")))
                .map(|_| node.layout().area)
        })
        .ok_or("source field")?;
    test.click_cursor((f64::from(field.min_x() + 3.), f64::from(field.min_y() + 3.)));
    test.write_text("unsaved ");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    let refused = submitted_route(&sender, target)?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert!(refused.try_recv()?.unwrap_err().contains("Source draft"));
    let other = linked_project()?;
    let cross_project = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &other.path().to_string_lossy())
            .append_pair("scene", "b.recite")
            .append_pair("beat", "other")
            .finish()
    );
    let refused_switch = submitted_route(&sender, cross_project)?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert!(refused_switch.try_recv()?.is_err());
    assert!(!contains_paragraph(&test, "Other scene."));
    assert!(!std::fs::read_to_string(dir.path().join("a.recite"))?.contains("unsaved"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn received_route_refuses_translation_drafts() -> Result<(), Box<dyn std::error::Error>> {
    let dir = linked_project()?;
    let root = dir.path().to_path_buf();
    std::fs::create_dir(root.join("locale"))?;
    let catalogue = root.join("locale/fr.po");
    let saved = "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\nmsgctxt \"7701ceab59d2adfa057a\"\nmsgid \"Would you tell me, please, which way I ought to go from here?\"\nmsgstr \"Bonjour\"\n";
    std::fs::write(&catalogue, saved)?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    let inbox = crate::ActivationInbox::new(receiver);
    let initial = format!(
        "recite://writer/translation?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &root.to_string_lossy())
            .append_pair("scene", "a.recite")
            .append_pair("catalogue", "locale/fr.po")
            .append_pair("entry", "7701ceab59d2adfa057a")
            .finish()
    );
    let target = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &root.to_string_lossy())
            .append_pair("scene", "b.recite")
            .finish()
    );
    let (mut test, _) = TestingRunner::new(
        crate::editor_app,
        Size2D::new(1400., 1000.),
        move |runner| {
            runner.provide_root_context(move || crate::InitialProject(Some(root)));
            runner.provide_root_context(move || InitialRoute(initial));
            runner.provide_root_context(move || inbox);
        },
        1.,
    );
    test.poll_n(std::time::Duration::from_millis(16), 40);
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|span| span.text == "Bonjour"))
                .map(|_| node.layout().area)
        })
        .ok_or("translation field")?;
    test.click_cursor((f64::from(field.max_x() - 3.), f64::from(field.center().y)));
    test.write_text(" !");
    test.poll_n(std::time::Duration::from_millis(16), 6);
    let refused = submitted_route(&sender, target)?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert!(
        refused
            .try_recv()?
            .unwrap_err()
            .contains("translation drafts")
    );
    let other = linked_project()?;
    let cross_project = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &other.path().to_string_lossy())
            .append_pair("scene", "b.recite")
            .finish()
    );
    let refused_switch = submitted_route(&sender, cross_project)?;
    test.poll_n(std::time::Duration::from_millis(16), 10);
    assert!(refused_switch.try_recv()?.is_err());
    assert!(!contains_paragraph(&test, "Other scene."));
    assert_eq!(std::fs::read_to_string(catalogue)?, saved);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn route_less_project_switch_clears_clean_catalogue() -> Result<(), Box<dyn std::error::Error>> {
    let first = linked_project()?;
    let second = linked_project()?;
    std::fs::create_dir(first.path().join("locale"))?;
    std::fs::write(
        first.path().join("locale/fr.po"),
        "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\nmsgctxt \"7701ceab59d2adfa057a\"\nmsgid \"Would you tell me, please, which way I ought to go from here?\"\nmsgstr \"Bonjour\"\n",
    )?;
    let initial = format!(
        "recite://writer/translation?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &first.path().to_string_lossy())
            .append_pair("scene", "a.recite")
            .append_pair("catalogue", "locale/fr.po")
            .append_pair("entry", "7701ceab59d2adfa057a")
            .finish()
    );
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    let inbox = crate::ActivationInbox::new(receiver);
    let root = first.path().to_path_buf();
    let (mut test, _) = TestingRunner::new(
        crate::editor_app,
        Size2D::new(1400., 1000.),
        move |runner| {
            runner.provide_root_context(move || crate::InitialProject(Some(root)));
            runner.provide_root_context(move || InitialRoute(initial));
            runner.provide_root_context(move || inbox);
        },
        1.,
    );
    test.poll_n(std::time::Duration::from_millis(16), 40);
    assert!(contains_paragraph(&test, "Bonjour"));
    let (reply, result) = std::sync::mpsc::channel();
    sender.try_send(crate::activation::IncomingRoute {
        project: Some(second.path().to_path_buf()),
        route: None,
        cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        reply,
    })?;
    test.poll_n(std::time::Duration::from_millis(16), 40);
    assert_eq!(result.try_recv()?, Ok(()));
    assert!(!contains_paragraph(&test, "Bonjour"));
    Ok(())
}
