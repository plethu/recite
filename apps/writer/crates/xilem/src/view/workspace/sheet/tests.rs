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
