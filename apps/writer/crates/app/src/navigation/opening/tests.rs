use super::*;
use crate::test_support;
use recite_writer_session::location::Location;

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";

fn link(document: &str) -> Location {
    Location {
        document: document.into(),
        ..Location::default()
    }
}

#[test]
fn links_to_documents_outside_the_project_are_refused() {
    let directory = test_support::project(SOURCE).unwrap();
    let mut writer = test_support::open(&directory).unwrap();
    assert_eq!(
        writer.follow_location(&link("elsewhere/missing.recite")),
        Err("The linked document is not in this project.".into())
    );
}

#[test]
fn links_to_unknown_examples_are_refused() {
    let mut writer = Writer::from_session(Session::examples().unwrap(), None);
    assert_eq!(
        writer.follow_location(&link("No such example")),
        Err("The linked example is unavailable.".into())
    );
}
