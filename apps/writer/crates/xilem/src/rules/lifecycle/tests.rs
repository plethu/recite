use super::*;

#[test]
fn retired_and_reopened_reply_rejects_a_receipt_even_at_the_same_source_and_structure() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.pick_rule_choice(selection::Target::AddCondition(None));
    let crate::choices::Request::Rules(receipt) = writer.pending_choice.take().unwrap() else {
        panic!("rule request")
    };
    let before = writer.session.model().navigation_stamp();
    writer.retire_rules();
    let mut projection = Some(
        writer
            .session
            .model()
            .document()
            .reply_rules("22222222222222222222")
            .unwrap(),
    );
    writer.install_rules(&mut projection);
    assert!(before.is_current(writer.session.model()));
    assert!(writer.rules_editable());
    assert!(!receipt.is_current(writer));
    let draft = writer.session.model().recovery();
    assert!(!receipt.apply(writer, selection::Choice::Argument("5".into())));
    assert_eq!(writer.session.model().recovery(), draft);
}

#[test]
fn only_the_latest_pending_choice_can_open_and_retirement_clears_it() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.pick_rule_choice(selection::Target::AddCondition(None));
    writer.choose_destination(recite_writer_model::RouteOwner::Reply(
        "22222222222222222222".into(),
    ));
    assert!(matches!(
        writer.pending_choice,
        Some(crate::choices::Request::Destination(_))
    ));
    writer.pick_rule_choice(selection::Target::AddEffect);
    assert!(matches!(
        writer.pending_choice,
        Some(crate::choices::Request::Rules(_))
    ));
    writer.retire_rules();
    assert!(writer.pending_choice.is_none());
}

#[test]
fn editing_reply_text_applies_checked_rule_draft_and_exits_source_backing_view() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = first_argument(writer).unwrap();
    writer.edit_rule_argument(target, "5".into());
    writer.edit_reply_text();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules.is_none());
    assert_eq!(
        writer.session.model().view(),
        &recite_writer_model::View::Passage("22222222222222222222".into())
    );
    assert_eq!(writer.current_location().screen, Screen::Write);
    assert_eq!(writer.session.model().draft(), "Let me through.");
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("standing(5)")
    );
}

#[test]
fn structural_authoring_and_reload_retire_rule_presentation_and_its_picker_receipts() {
    for reload in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.pick_rule_choice(selection::Target::AddCondition(None));
        let receipt = match writer.pending_choice.take().unwrap() {
            crate::choices::Request::Rules(request) => request,
            _ => panic!("expected a rule request"),
        };
        if reload {
            writer.reload();
        } else {
            writer.change(recite_writer_model::Workbench::add_beat);
        }
        assert!(writer.rules.is_none());
        assert!(!receipt.is_current(writer));
        assert_eq!(writer.current_location().screen, Screen::Write);
    }
}

#[test]
fn undo_removing_the_open_reply_retires_controls_and_preserves_redo() {
    let directory =
        support::project(":: start default\n> line@11111111111111111111\n  Begin.\n-> END\n")
            .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = tokio::sync::mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.change(recite_writer_model::Workbench::add_choice);
    let recite_writer_model::View::Passage(passage) = writer.session.model().view().clone() else {
        panic!("new reply must be selected")
    };
    writer.navigate(Target::Location(Location {
        screen: Screen::Rules,
        passage: Some(passage),
        ..Location::default()
    }));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.rules.is_some());
    writer.navigate(Target::Edit(EditOperation::Undo));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.rules.is_none());
    assert_eq!(writer.current_location().screen, Screen::Write);
    assert!(writer.current_location().source);
    assert!(!writer.status.is_empty());
    assert!(
        !writer
            .session
            .model()
            .document()
            .source()
            .contains("? reply")
    );
    assert!(writer.session.model().document().can_redo());
    writer.navigate(Target::Edit(EditOperation::Redo));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.session.model().document().source().contains("? "));
}

#[test]
fn destination_change_from_rules_applies_typed_arguments_then_retires_old_destination_controls() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = first_argument(writer).unwrap();
    writer.edit_rule_argument(target, "5".into());
    writer.choose_destination(recite_writer_model::RouteOwner::Reply(
        "22222222222222222222".into(),
    ));
    let crate::choices::Request::Destination(request) = writer.pending_choice.take().unwrap()
    else {
        panic!("destination request")
    };
    request.apply(writer, "END".into());
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules.is_none());
    assert_eq!(
        writer.session.model().view(),
        &recite_writer_model::View::Passage("22222222222222222222".into())
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("standing(5)")
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Let me through.\n  -> END")
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("# Keep effect spacing.")
    );
    assert!(!writer.session.model().has_draft());
}
