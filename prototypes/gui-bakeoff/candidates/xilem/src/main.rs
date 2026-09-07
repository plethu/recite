use recite_bakeoff_authoring::{FIXTURE, View, Workbench, WorkbenchError};
use xilem::masonry::properties::types::Length;
use xilem::{
    Color, EventLoop, InsertNewline, WidgetView, WindowOptions, Xilem,
    style::Style,
    view::{flex_col, flex_row, label, portal, sized_box, text_button, text_input},
};

struct State {
    model: Workbench,
    status: String,
    dark: bool,
}
impl State {
    fn action(&mut self, f: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>) {
        self.status = match f(&mut self.model) {
            Ok(()) => "Applied. Changes are temporary.".into(),
            Err(e) => e.to_string(),
        };
    }
}
fn app(state: &mut State) -> impl WidgetView<State> + use<> {
    let fg = if state.dark {
        Color::from_rgb8(232, 228, 222)
    } else {
        Color::from_rgb8(52, 50, 46)
    };
    let bg = if state.dark {
        Color::from_rgb8(32, 33, 36)
    } else {
        Color::from_rgb8(245, 242, 237)
    };
    let passages = state.model.document().passages().unwrap_or_default();
    let navigation = passages
        .iter()
        .map(|p| {
            let id = p.id.clone();
            text_button(p.section.clone(), move |s: &mut State| {
                s.action(|m| m.select(View::Passage(id.clone())))
            })
        })
        .collect::<Vec<_>>();
    let context = passages
        .into_iter()
        .map(|p| label(p.text).text_size(21.).color(fg))
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            flex_row((
                label("recite.").text_size(30.).color(fg),
                text_button("Script", |s: &mut State| s.action(Workbench::show_script)),
                text_button("Source", |s: &mut State| {
                    s.action(|m| m.select(View::Source))
                }),
                text_button("Undo", |s: &mut State| s.action(Workbench::undo)),
                text_button("Redo", |s: &mut State| s.action(Workbench::redo)),
                text_button("Light / dark", |s: &mut State| s.dark = !s.dark),
            ))
            .gap(Length::px(10.)),
            label(state.status.clone()).color(fg),
            flex_row((
                sized_box(flex_col(navigation).gap(Length::px(10.))).width(Length::px(180.)),
                sized_box(
                    flex_col((
                        label(if state.model.view() == &View::Source {
                            "Source"
                        } else {
                            "Selected passage"
                        })
                        .color(fg),
                        sized_box(
                            text_input(state.model.draft().to_owned(), |s: &mut State, text| {
                                s.model.set_draft(text)
                            })
                            .insert_newline(InsertNewline::OnEnter)
                            .text_color(fg),
                        )
                        .width(Length::px(850.))
                        .height(Length::px(240.)),
                        flex_row((
                            text_button("Apply draft", |s: &mut State| s.action(Workbench::apply)),
                            text_button("Discard draft", |s: &mut State| {
                                s.action(|m| {
                                    m.discard();
                                    Ok(())
                                })
                            }),
                            text_button("Add choice", |s: &mut State| {
                                s.action(Workbench::add_choice)
                            }),
                        ))
                        .gap(Length::px(10.)),
                        portal(flex_col(context).gap(Length::px(16.))),
                    ))
                    .gap(Length::px(16.)),
                )
                .width(Length::px(880.)),
            ))
            .gap(Length::px(24.)),
        ))
        .gap(Length::px(20.)),
    )
    .expand()
    .background_color(bg)
    .padding(24.)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = State {
        model: Workbench::new(FIXTURE)?,
        status: "Choose a passage. Changes are temporary.".into(),
        dark: false,
    };
    Xilem::new_simple(
        state,
        app,
        WindowOptions::new(
            std::env::var("RECITE_BAKEOFF_WINDOW_ID")
                .unwrap_or_else(|_| "Recite — Xilem bake-off".into()),
        )
        .with_initial_inner_size(xilem::dpi::LogicalSize::new(1200., 800.)),
    )
    .run_in(EventLoop::with_user_event())?;
    Ok(())
}
