use super::*;

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
