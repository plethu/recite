use super::*;

#[test]
fn five_hundred_twelve_choices_mount_sixteen_controls_and_keep_runtime_indices() {
    use std::fmt::Write as _;
    let mut source = SOURCE.replace("-> END\n", "");
    for index in 0..512 {
        writeln!(
            source,
            "? reply@{:020}\n  Choice {index:03}\n  -> END",
            index + 2
        )
        .unwrap();
    }
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    writer.project.current_mut().advance_preview(None).unwrap();
    assert_eq!(
        writer
            .project
            .current()
            .model()
            .preview_page()
            .unwrap()
            .choices
            .len(),
        512
    );
    let mounted = labels(&mut writer).unwrap();
    assert_eq!(
        mounted
            .iter()
            .filter(|label| label.strip_prefix("Choice ").is_some_and(
                |number| number.len() == 3 && number.bytes().all(|byte| byte.is_ascii_digit())
            ))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
    click(&mut writer, &["Next preview choices"]).unwrap();
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Choice 016")
    );
    click(
        &mut writer,
        &[
            "Previous preview choices",
            "Next preview choices",
            "Choice 016",
        ],
    )
    .unwrap();
    assert!(
        writer
            .project
            .current()
            .model()
            .preview_page()
            .unwrap()
            .ended
    );
    assert_eq!(writer.trial.choice_page, 0);
}

#[test]
fn deferred_effect_pages_keep_every_request_available_without_mounting_the_whole_trace() {
    use std::fmt::Write as _;
    let mut source = SOURCE.replace("-> END\n", "");
    for index in 0..33 {
        writeln!(
            source,
            "! deferred effect_{index}(work, \"quoted\", 7, 1.5, true)"
        )
        .unwrap();
    }
    source.push_str("-> END\n");
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    for _ in 0..34 {
        writer.project.current_mut().advance_preview(None).unwrap();
        if writer
            .project
            .current()
            .model()
            .preview_page()
            .unwrap()
            .ended
        {
            break;
        }
    }
    assert_eq!(
        writer
            .project
            .current()
            .model()
            .preview_page()
            .unwrap()
            .effects
            .len(),
        33
    );
    let mounted = labels(&mut writer).unwrap();
    assert_eq!(
        mounted
            .iter()
            .filter(|label| label.starts_with("Deferred effect request"))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
    assert!(
        mounted
            .iter()
            .any(|label| label
                == "Deferred effect request · effect_0(work, \"quoted\", 7, 1.5, true)")
    );
    click(
        &mut writer,
        &["Next effect requests", "Next effect requests"],
    )
    .unwrap();
    let mounted = labels(&mut writer).unwrap();
    assert_eq!(
        mounted
            .iter()
            .filter(|label| label.starts_with("Deferred effect request"))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1
    );
    assert!(mounted.iter().any(|label| label.contains("effect_32(")));
    click(&mut writer, &["Previous effect requests"]).unwrap();
    assert_eq!(writer.trial.effect_page, 1);
}
