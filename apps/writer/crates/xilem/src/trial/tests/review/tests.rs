use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use recite_core::ast::InterpolationType;
use std::fs;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn reviewing_a_failed_restart_targets_current_values_and_preserves_its_diagnostic() {
    let bindings = (0..6)
        .map(|index| format!("bind=(v{index}:int=$input{index})"))
        .collect::<Vec<_>>()
        .join(" ");
    let text = (0..6)
        .map(|index| format!("{{v{index}}}"))
        .collect::<Vec<_>>()
        .join(" ");
    let directory = project(&format!(
        ":: start default\n> {bindings} @ffee6p8\n  {text}\n-> END\n"
    ))
    .unwrap();
    let mut writer = open(&directory).unwrap();
    for index in 0..6 {
        writer
            .trial
            .settings
            .values
            .insert(format!("input{index}"), "1".into());
    }
    install(&mut writer).unwrap();
    writer.configure_trial();
    let mut mounted = Mounted::new(writer, surface);
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    mounted.state.trial.sender = Some(sender);
    mounted
        .state
        .trial
        .settings
        .values
        .insert("input5".into(), "invalid".into());
    mounted.settle();
    mounted.activate("Restart preview");
    assert!(
        finish(&mut mounted.state, requests.try_recv().unwrap())
            .unwrap()
            .is_none()
    );
    mounted.settle();
    let failure = mounted.state.trial.failure().unwrap().to_owned();
    assert!(failure.starts_with("input5:"));
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "1 1 1 1 1 1"
    );
    mounted.activate("Review current inputs");
    assert_eq!(mounted.state.trial.settings.binding_page, 1);
    assert!(
        matches!(&mounted.state.trial.settings.review, Review::Value { value, kind: InterpolationType::Integer } if value == "input5")
    );
    assert!(mounted.contains_text("Preview value: input5"));
    assert!(!mounted.contains_text("Preview value: input0"));
    mounted.state.trial.settings.values.remove("input5");
    mounted.activate("Review current inputs");
    assert!(
        mounted.contains_text("Enter a whole number."),
        "explicit review exposes a missing numeric value too"
    );
    mounted
        .state
        .trial
        .settings
        .values
        .insert("input5".into(), "invalid".into());
    mounted.activate("Restart preview");
    let request = requests.try_recv().unwrap();
    mounted
        .state
        .trial
        .settings
        .values
        .insert("input5".into(), "2".into());
    assert!(finish(&mut mounted.state, request).unwrap().is_none());
    mounted.settle();
    mounted.activate("Review current inputs");
    assert!(
        matches!(mounted.state.trial.settings.review, Review::Diagnostic),
        "corrected values must not be blamed for the failed captured attempt"
    );
    mounted.replace_text("Preview value: input5", "invalid again");
    assert!(mounted.contains_text("Values were valid when checked."));
    mounted.replace_text("Preview value: input5", "2");
    mounted.activate("Continue preview");
    assert!(mounted.state.session.model().preview_page().unwrap().ended);
    assert_eq!(
        mounted.state.trial.failure(),
        Some(failure.as_str()),
        "playback status cannot erase the failed attempt's diagnostic"
    );
    mounted.activate("Hide preview error");
    assert!(matches!(
        mounted.state.trial.settings.review,
        Review::Inputs
    ));
    mounted.activate("Restart preview");
    assert!(matches!(
        finish(&mut mounted.state, requests.try_recv().unwrap()).unwrap(),
        Some(Retired::Preview(_))
    ));
    assert!(!mounted.state.trial.failed());
    assert!(mounted.state.trial.feedback(true).is_none());
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "1 1 1 1 1 2"
    );
}

#[test]
fn review_with_valid_scalars_exposes_the_actual_preparation_diagnostic() {
    let directory = project(":: start default\n> bind=(flag:bool=$flag) bind=(person:string=$person) @ffee6p8\n  {flag} {person}\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    let request = begin(&mut writer).unwrap();
    let (receipt, returned) = mpsc::sync_channel(1);
    writer.accept_trial(Output {
        request,
        result: Err("A compiler diagnostic with valid scalar inputs.".into()),
        receipt,
    });
    drop(returned.recv().unwrap());
    writer.review_trial_inputs();
    assert!(
        matches!(writer.trial.settings.review, Review::Diagnostic),
        "absent Boolean uses false and absent String stays valid"
    );
    assert_eq!(
        writer.trial.failure(),
        Some("A compiler diagnostic with valid scalar inputs.")
    );
    assert!(!writer.trial.busy());
}

#[test]
fn document_replacement_clears_terminal_feedback_and_preserves_in_flight_occupancy() {
    let directory = project(SOURCE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(
        &other,
        ":: other\n> @rthh7ab\n  Other conversation.\n-> END\n",
    )
    .unwrap();
    let mut writer = open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    install(&mut writer).unwrap();
    writer.switch(&other);
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    writer
        .session
        .edit(|model| model.select(recite_writer_model::View::Block("other".into())))
        .unwrap();
    install(&mut writer).unwrap();
    writer.switch(&directory.path().join("scene.recite"));
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    writer.trial.sender = None;
    writer.start_trial();
    assert!(writer.trial.failed());
    writer.review_trial_inputs();
    assert!(matches!(writer.trial.settings.review, Review::Diagnostic));
    writer.switch(&other);
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    writer.configure_trial();
    assert!(!writer.trial.failed());
    assert!(writer.trial.feedback(true).is_none());
    assert!(matches!(writer.trial.settings.review, Review::Inputs));
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Other conversation."
    );
    writer.switch(&directory.path().join("scene.recite"));
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    let request = begin(&mut writer).unwrap();
    writer.switch(&other);
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    assert!(
        writer.trial.busy(),
        "a file switch cannot free the active worker slot"
    );
    assert!(matches!(
        finish(&mut writer, request).unwrap(),
        Some(Retired::Prepared(_))
    ));
    assert!(!writer.trial.failed());
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Other conversation."
    );
}
