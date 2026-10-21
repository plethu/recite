use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted, workspaces::tests::ToolDriver};
use recite_writer_session::preview::trace::{Projection, Section};
use std::fs;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_preview_inputs_and_trace_preserve_the_old_run_until_a_valid_restart() {
    let argument = "A long captured condition argument remains inspectable. ".repeat(60);
    let source = format!(
        ":: start default\n:if accepts(\"{argument}\", actor, other, third, fourth)\n  > tickets@12345678901234567890 bind=(count:int=$a_count)\n    {{count}} ticket\n    | {{count}} tickets\n> details@11111111111111111111 bind=(person:string=$b_person) bind=(ratio:float=$c_ratio) bind=(flag:bool=$d_flag) bind=(tail:string=$e_tail)\n  {{person}} {{ratio}} {{flag}} {{tail}}\n-> END\n"
    );
    let directory = project(&source).unwrap();
    let po = directory.path().join("fr.po");
    fs::write(&po, "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"12345678901234567890\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.open_localisation();
    let mut mounted = Mounted::new(writer, surface);
    let mut tools = ToolDriver::new(&mut mounted.state);
    mounted.type_text("Catalogue path", po.to_str().unwrap());
    mounted.activate("Open catalogue");
    tools.finish(&mut mounted.state);
    mounted.state.return_to_writing();
    mounted.settle();
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    mounted.state.trial.sender = Some(sender);
    mounted.activate("Preview inputs");
    mounted.type_text("Find preview language", "fr-CA");
    mounted.activate("Search preview languages");
    mounted.activate("Preview in French (Canada) · fr-CA");
    mounted.type_text("Preview wording", "formal");
    mounted.type_text("Preview value: a_count", "2");
    mounted.type_text("Preview value: b_person", "Ada");
    mounted.type_text("Preview value: c_ratio", "1.5");
    mounted.activate("Preview value: d_flag");
    mounted.activate("More preview values");
    mounted.type_text("Preview value: e_tail", "Retained across pages");
    mounted.activate("Previous preview values");
    mounted.activate("Start / restart preview");
    finish(&mut mounted.state, requests.try_recv().unwrap()).unwrap();
    mounted.settle();
    mounted.activate("Answer true");
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "2 billets"
    );
    assert!(
        mounted
            .state
            .session
            .model()
            .preview_provenance()
            .unwrap()
            .caption
            .contains("fr-CA")
    );
    mounted.activate("Preview trace");
    let Projection::List(list) = mounted
        .state
        .trial
        .trace
        .project(mounted.state.session.model().preview_trace().unwrap())
    else {
        panic!("trace list")
    };
    let condition = list
        .rows
        .iter()
        .find(|row| row.summary.contains("Condition requested"))
        .unwrap();
    mounted.activate(&format!(
        "Open trace record {} · {}",
        condition.index + 1,
        condition.summary
    ));
    mounted.activate("Read complete trace value · Argument 1 · String");
    let original_cursor = cursor(mounted.state.trial.trace.place);
    mounted.activate("More trace value");
    assert!(matches!(
        mounted.state.trial.trace.place,
        Place::Value(ValuePosition { part: 1, .. })
    ));
    mounted.activate("Previous trace value part");
    assert_eq!(cursor(mounted.state.trial.trace.place), original_cursor);
    mounted.replace_text("Preview value: a_count", "not a count");
    mounted.activate("Start / restart preview");
    assert!(
        finish(&mut mounted.state, requests.try_recv().unwrap())
            .unwrap()
            .is_none()
    );
    assert!(mounted.state.trial.status.contains("unchanged"));
    assert_eq!(cursor(mounted.state.trial.trace.place), original_cursor);
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "2 billets"
    );
    mounted.replace_text("Preview value: a_count", "1");
    mounted.state.change(|model| {
        model.set_draft("Unapplied manuscript.".into());
        Ok(())
    });
    mounted.settle();
    mounted.activate("Start / restart preview");
    assert!(requests.try_recv().is_err());
    assert_eq!(cursor(mounted.state.trial.trace.place), original_cursor);
    mounted.state.change(|model| {
        model.discard();
        Ok(())
    });
    mounted.settle();
    mounted.activate("Return to trace record");
    mounted.activate("More record details");
    mounted.activate("Previous record details");
    mounted.activate("Return to trace records");
    mounted.activate("Trace: Plural lookups");
    assert!(matches!(
        mounted.state.trial.trace.section,
        Section::Plurals
    ));
    mounted.activate("Trace: Translation lookups");
    mounted.activate("Start / restart preview");
    assert!(matches!(
        finish(&mut mounted.state, requests.try_recv().unwrap()).unwrap(),
        Some(Retired::Preview(_))
    ));
    assert!(matches!(
        mounted.state.trial.trace.place,
        Place::List { page: 0 }
    ));
    mounted.settle();
    mounted.activate("Answer true");
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "1 billet"
    );
    mounted.activate("Continue preview");
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "Ada 1.5 true Retained across pages"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
}

fn cursor(place: Place) -> (usize, usize, usize) {
    let Place::Value(position) = place else {
        panic!("complete trace value")
    };
    (position.record, position.field, position.part)
}
