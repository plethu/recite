use super::*;
use crate::presentation::grid::DESK_SIDE;

fn writer(width: f64) -> Writer {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer.viewport.width = width;
    writer
}

#[test]
fn script_is_a_sheet_on_the_desk_only_where_its_measure_fits() {
    let mut writer = writer(1200.);
    let layout = Layout::resolve(&writer);
    assert!(layout.grid.sheet);
    assert_eq!(layout.script, layout.grid.row());
    assert!(layout.grid.width < layout.script_pane);
    writer.map.workspace = Workspace::Split;
    assert!(!Layout::resolve(&writer).grid.sheet, "Split is full-bleed");
    writer.map.workspace = Workspace::Map;
    assert!(!Layout::resolve(&writer).grid.sheet);
    // Below 900 logical pixels, and where the measure would fall under 480px, Script is
    // full-bleed.
    assert!(!Layout::resolve(&self::writer(899.)).grid.sheet);
    assert!(!Layout::resolve(&self::writer(900.)).grid.sheet);
}

#[test]
fn at_200_percent_the_sheet_zooms_and_its_text_reflows_inside_the_pane() {
    let mut writer = writer(1200.);
    let normal = Layout::resolve(&writer).grid;
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let zoomed = Layout::resolve(&writer);
    // The columns are set in ems, so they double; the measure gives way to the pane.
    assert!((zoomed.grid.margin - 2. * normal.margin).abs() < 1e-9);
    assert!(zoomed.grid.measure > 0. && zoomed.grid.row() <= zoomed.script_pane);
    // The paper fills the pane, less its desk either side where it rests on the desk.
    let desk = 2. * DESK_SIDE;
    assert!(zoomed.grid.width >= zoomed.script_pane - desk - 1e-6);
}

#[test]
fn tool_pages_set_their_content_on_the_sheet_where_script_rests_on_one() {
    let mut writer = writer(1200.);
    // Writing keeps the whole content width; Script's sheet is inside it.
    assert_eq!(page_width(&writer), writer.content_width());
    writer.workspaces.screen = crate::workspaces::Screen::Build;
    let grid = page_grid(&writer);
    assert!(grid.sheet);
    assert_eq!(
        grid,
        Layout::resolve(&writer).grid,
        "the tool sheet is Script's"
    );
    // Inside the sheet's rule edges and its pad either side.
    assert_eq!(page_width(&writer), grid.width - 2. * (grid.pad + 1.));
    // Where Script goes full-bleed, so do tool pages, inside the workspace's padding.
    let mut narrow = self::writer(900.);
    narrow.workspaces.screen = crate::workspaces::Screen::Build;
    assert!(!page_grid(&narrow).sheet);
    assert_eq!(page_width(&narrow), narrow.content_width());
}
