use super::prepare::Inputs;
use crate::localisation::catalogue::Catalogue;
use recite_writer_model::Workbench;
use recite_writer_session::preview::Request;

pub(super) fn capture(
    model: &Workbench,
    catalogue: Option<&Catalogue>,
    localised: bool,
) -> Request {
    Request::capture(
        model,
        Inputs {
            locale: if localised {
                "fr".into()
            } else {
                String::new()
            },
            ..Inputs::default()
        },
        catalogue,
    )
    .unwrap()
}

use recite_writer_model::View;

#[test]
fn publication_rejects_departure_reopen_widget_draft_and_catalogue_change()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> guide@12345678901234567890\n  Hello.\n-> END\n";
    let mut model = Workbench::new(source)?;
    model.select(View::Source)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "msgctxt \"12345678901234567890\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    let mut catalogue = Catalogue::open(&path)?;
    let requested = capture(&model, Some(&catalogue), true);
    let source_only = capture(&model, Some(&catalogue), false);
    assert!(requested.is_current(&model, Some(&catalogue)));
    let mut reopened = Workbench::new(source)?;
    reopened.select(View::Source)?;
    assert!(!requested.is_current(&reopened, Some(&catalogue)));
    // Same widget harvest called by the actual result task, before its predicate.
    model.select(View::Source)?;
    crate::buffers::harvest_draft(
        &mut model,
        &freya::code_editor::Rope::from_str(&format!("{source}# draft\n")),
        "",
    );
    assert!(!requested.is_current(&model, Some(&catalogue)));
    model.discard();
    catalogue.reload()?;
    assert!(!requested.is_current(&model, Some(&catalogue)));
    assert!(source_only.is_current(&model, Some(&catalogue)));
    Ok(())
}
