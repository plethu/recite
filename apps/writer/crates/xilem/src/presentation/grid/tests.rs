use super::*;
use crate::presentation::tokens::size;

#[test]
fn at_1200_by_800_the_sheet_takes_the_pane_less_24px_of_desk_and_keeps_every_column() {
    // 1200 less the 224px scene list and the workspace's 16px padding either side.
    let grid = Grid::resolve(1200. - 224. - 32., (20., 1.), true);
    assert!(grid.sheet);
    assert_eq!(grid.width, 928.);
    assert_eq!((grid.margin, grid.destination), (96., 160.));
    assert_eq!(grid.measure, 928. - 64. - 96. - 16. - 24. - 160.);
    assert_eq!(grid.pad, 32.);
    assert!((grid.pad * 2. + grid.row() - grid.width).abs() < 1e-9);
}

#[test]
fn at_900_by_650_the_paper_goes_full_bleed_and_destinations_wrap_under_a_short_measure() {
    let grid = Grid::resolve(900. - 192. - 32., (20., 1.), true);
    assert!(!grid.sheet);
    assert_eq!(grid.margin, 72.);
    assert!(grid.measure >= DESTINATION_MEASURE || grid.destination == 0.);
    // Beside a destination column the measure would be 372px, so destinations wrap.
    assert_eq!(grid.destination, 0.);
    assert_eq!(grid.measure, 676. - 32. - 72. - 16.);
}

#[test]
fn the_measure_never_exceeds_68_characters_and_the_desk_shows_either_side_of_the_sheet() {
    let grid = Grid::resolve(2400., (20., 1.), true);
    assert!(grid.sheet);
    assert!((grid.measure - 68. * 0.46 * 20.).abs() < 1e-9);
    // About 32em, inside the 640px measure interface prose keeps.
    assert!(grid.measure <= size::MEASURE);
    // The sheet holds its columns and pad; the rest of the pane is desk, centred around it.
    assert_eq!(grid.pad, 32.);
    assert!((grid.pad * 2. + grid.row() - grid.width).abs() < 1e-9);
    assert!(grid.width < 2400. - 2. * DESK_SIDE);
    // The destination column keeps room for its button's padding at 200%.
    let doubled = Grid::resolve(2400., (20., 2.), true);
    assert_eq!(
        doubled.destination,
        grid.destination + 2. * size::CONTROL_PAD
    );
    // A larger reading size widens every sheet column with it.
    let larger = Grid::resolve(2400., (28., 1.), true);
    assert!(larger.margin > grid.margin && larger.measure > grid.measure);
    // At the smallest reading size destinations hold their 13px floor, and so does their column.
    let smallest = Grid::resolve(2400., (14., 1.), true);
    assert!((smallest.destination - (134. * 13. / 14.4 + 26.)).abs() < 1e-3);
    assert!(smallest.margin < grid.margin);
}

#[test]
fn without_room_for_a_480px_measure_or_beside_the_map_the_paper_is_full_bleed() {
    assert!(
        !Grid::resolve(1000., (20., 1.), false).sheet,
        "Split is full-bleed"
    );
    let narrow = Grid::resolve(800., (20., 1.), true);
    assert!(!narrow.sheet);
    let tiny = Grid::resolve(300., (20., 1.), true);
    assert_eq!((tiny.margin, tiny.gutter()), (0., 0.));
    assert!(tiny.measure > 0.);
}
