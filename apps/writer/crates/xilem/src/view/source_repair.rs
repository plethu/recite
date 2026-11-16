//! Source drafts are repaired in a real editor, then imported through checked Apply. The page is a
//! banner that says what needs repair, with Reload and the import row it opens, over a head whose
//! one Primary opens the source in the editor, or applies a staged draft.
use super::*;
use crate::WriterLayout;
use crate::presentation::banner::banner;
use crate::presentation::print;
use crate::presentation::space;
use crate::presentation::{Type, Typeset};
use recite_writer_model::EditOperation;
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::{LineBreaking, TextAreaLabel};
use xilem::view::FlexSequence;

/// The repair page: `importing` when a recovered draft waits for its repaired text, otherwise the
/// accepted source has a syntax `error` to repair in the editor. One muted caption says what to
/// do; a staged draft's Apply and Discard join the head, and Apply is then its one Primary.
pub(super) fn panel(
    state: &Writer,
    importing: bool,
    error: Option<&str>,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = state.script_width();
    let caption = error.map_or_else(
        || {
            if importing {
                "Export opens the recovered draft in your editor; Import checks the repaired \
                 text, and Save writes it."
            } else {
                "Repair the source in your editor, then Reload. Writer keeps a recovery copy."
            }
            .to_owned()
        },
        str::to_owned,
    );
    let staged = state.session.model().has_draft();
    flex_col((
        repair_banner(state, importing),
        flex_row((
            flex_col((
                Type::Display
                    .label(scale, "Repair source")
                    .prop(LineBreaking::WordWrap),
                print::meta(scale, caption)
                    .color(state.palette.muted)
                    .prop(LineBreaking::WordWrap),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::UNIT.px())
            .flex(1.),
            open(state, importing, !staged),
            staged.then(|| draft(state)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::SIBLINGS.px())
        .width(width.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}

/// A staged draft's Discard, quiet, and Apply, the head's one Primary while the draft exists.
fn draft(state: &Writer) -> impl FlexSequence<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing || state.session.read_only();
    (
        print::action(scale, "Discard draft", closing, |state: &mut Writer| {
            state.change(|model| {
                model.discard();
                Ok(())
            })
        }),
        print::primary_action(
            scale,
            "Apply draft",
            &state.palette,
            closing || state.navigation.busy(),
            |state: &mut Writer| {
                state.navigate(crate::navigation::Target::Edit(EditOperation::Apply))
            },
        ),
    )
}

/// Open in editor: export the recovered draft and open it, open the exported draft again, or
/// open the accepted source. It is the head's Primary unless a staged draft's Apply is.
fn open(state: &Writer, importing: bool, primary: bool) -> impl WidgetView<Writer> + use<> {
    let busy = repair_busy(state);
    let project = state.session.project();
    let exported = state.repair.artifact.as_ref().is_some_and(|artifact| {
        project.is_some_and(|project| project.current().path() == artifact.document)
    });
    let action: fn(&mut Writer) = match (importing, exported) {
        (true, false) => Writer::export_source_repair,
        (true, true) => Writer::open_exported_source,
        (false, _) => Writer::open_editor,
    };
    let disabled = busy || project.is_none() || state.external.busy();
    if primary {
        Either::A(print::primary_action(
            state.ui_scale(),
            "Open in editor",
            &state.palette,
            disabled,
            action,
        ))
    } else {
        Either::B(print::action(
            state.ui_scale(),
            "Open in editor",
            disabled,
            action,
        ))
    }
}

fn repair_busy(state: &Writer) -> bool {
    state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy()
}

/// What needs repair, with Reload and, for a recovered draft, the import row it opens.
fn repair_banner(state: &Writer, importing: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let busy = repair_busy(state);
    let width = state.script_width();
    let status = if !importing {
        "The accepted source has syntax problems"
    } else if state.source_repair_pending() {
        "Import is required before Save"
    } else if state.session.current_dirty() {
        "Repaired source accepted · Unsaved"
    } else {
        "Accepted source · Saved"
    };
    let open = importing && state.repair.importing;
    let message = flex_col((
        flex_row((
            Type::Control
                .label(scale, "Source needs repair")
                .weight(FontWeight::SEMI_BOLD),
            print::meta(scale, status).color(state.palette.muted),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::SIBLINGS.px()),
        open.then(|| import_row(state, busy)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::SIBLINGS.px());
    let actions = flex_row((
        text_button(scale, "Reload", Writer::reload)
            .accessible_label("Reload (keep recovery copy)")
            .disabled(busy || state.session.project().is_none()),
        importing.then(|| {
            text_button(
                scale,
                "Import repaired source…",
                Writer::toggle_source_repair_import,
            )
            .expanded(open)
            .disabled(busy)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px());
    banner(
        &state.palette,
        (message, actions),
        open || width < 560. * f64::from(scale),
        width,
    )
}

/// The repaired file's path, Browse and Import on one line inside the banner.
fn import_row(state: &Writer, busy: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        keyed(
            state.session.model().document().key().clone(),
            text_input(state.repair_path().to_owned(), Writer::edit_repair_path)
                .typeset(Type::Body, scale)
                .placeholder("Repaired source file")
                .on_enter(|state: &mut Writer, _| state.import_source_repair())
                .disabled(busy)
                .prop(TextAreaLabel::new("Repaired source file")),
        )
        .flex(1.),
        text_button(scale, "Browse…", Writer::choose_source_repair).disabled(busy),
        text_button(
            scale,
            "Import repaired source",
            Writer::import_source_repair,
        )
        .disabled(busy || state.repair_path().trim().is_empty()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
}
