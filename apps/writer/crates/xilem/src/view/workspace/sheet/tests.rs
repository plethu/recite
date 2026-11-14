use super::*;

fn writer(width: f64) -> Writer {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer.viewport.width = width;
    writer
}

#[test]
fn script_is_a_sheet_on_the_desk_only_where_one_fits() {
    let mut writer = writer(1200.);
    let layout = Layout::resolve(&writer);
    assert!(layout.sheet);
    assert!(layout.script + EXTENT <= layout.script_pane);
    writer.map.workspace = Workspace::Split;
    assert!(!Layout::resolve(&writer).sheet, "Split is full-bleed");
    writer.map.workspace = Workspace::Map;
    assert!(!Layout::resolve(&writer).sheet);
    // Below 900 logical pixels, and where margins and shadow would not fit, Script is full-bleed.
    assert!(!Layout::resolve(&self::writer(899.)).sheet);
    let narrow = self::writer(1000.);
    let layout = Layout::resolve(&narrow);
    assert!(!layout.sheet && layout.script + EXTENT > layout.script_pane);
}
