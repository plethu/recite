//! The Script sheet's grid. Across the paper: a pad, the margin column (speaker labels, reply
//! numbers and margin marks), a gutter, the measure, a gutter and the destination column, then the
//! pad again. The sheet rests on the desk where its measure keeps at least 480px; otherwise the
//! paper goes full-bleed, its margin narrows and destinations wrap under their replies.
#[cfg(test)]
mod tests;

/// Desk either side of a sheet, beyond the workspace's own 16px padding: 24px in all.
pub(crate) const DESK_SIDE: f64 = 8.;
/// Paper either side of the columns on a sheet, and on full-bleed paper.
const SHEET_PAD: f64 = 32.;
const BLEED_PAD: f64 = 16.;
/// Between the margin column and the measure, and between the measure and destinations.
pub(crate) const GUTTER: f64 = 16.;
pub(crate) const DESTINATION_GUTTER: f64 = 24.;
/// The least measure a sheet keeps, and the least beside a destination column.
const SHEET_MEASURE: f64 = 480.;
const DESTINATION_MEASURE: f64 = 420.;
/// A route button's horizontal padding at 100%.
pub(crate) const ROUTE_PADDING: f64 = 12.;
/// The longest measure: about 68 characters of the reading face.
const CHARACTERS: f64 = 68.;
const CHARACTER: f64 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Grid {
    /// A sheet on the desk, or full-bleed paper.
    pub sheet: bool,
    /// The paper's whole width.
    pub width: f64,
    /// Paper before the margin column; the rest of the width lies after the columns.
    pub pad: f64,
    pub margin: f64,
    pub measure: f64,
    /// Zero where destinations wrap under their replies.
    pub destination: f64,
}

impl Grid {
    /// The grid for paper `available` wide at reading size `reading`. `sheet` says whether a
    /// sheet may rest on the desk at all, as it may in Script but not beside the map.
    pub fn resolve(available: f64, (reading, scale): (f32, f32), sheet: bool) -> Self {
        // Sheet columns are set with the manuscript: in proportion to its reading size. The
        // destination column also holds its route button's padding, which scales with the UI.
        let unit = f64::from(reading) / 20.;
        let longest = CHARACTERS * CHARACTER * f64::from(reading);
        let destination = 160. * unit + 2. * ROUTE_PADDING * (f64::from(scale) - 1.).max(0.);
        let width = available - 2. * DESK_SIDE;
        let measure =
            width - 2. * SHEET_PAD - 96. * unit - GUTTER - DESTINATION_GUTTER - destination;
        if sheet && measure >= SHEET_MEASURE {
            let measure = measure.min(longest);
            let columns = 96. * unit + GUTTER + measure + DESTINATION_GUTTER + destination;
            return Self {
                sheet: true,
                width,
                // Paper the measure cap leaves over is shared either side, so the text stays
                // centred on the sheet.
                pad: (width - columns) / 2.,
                margin: 96. * unit,
                measure,
                destination,
            };
        }
        let margin = if available >= 360. { 72. * unit } else { 0. };
        let gutter = if margin > 0. { GUTTER } else { 0. };
        let beside =
            available - 2. * BLEED_PAD - margin - gutter - DESTINATION_GUTTER - destination;
        let (measure, destination) = if beside >= DESTINATION_MEASURE {
            (beside, destination)
        } else {
            (available - 2. * BLEED_PAD - margin - gutter, 0.)
        };
        let measure = measure.clamp(0., longest);
        let columns = margin + gutter + measure + Self::trailing(destination);
        Self {
            sheet: false,
            width: available,
            pad: BLEED_PAD.max((available - columns) / 2.),
            margin,
            measure,
            destination,
        }
    }
    fn trailing(destination: f64) -> f64 {
        if destination > 0. {
            DESTINATION_GUTTER + destination
        } else {
            0.
        }
    }
    /// The gutter after the margin column, absent with the column.
    pub fn gutter(&self) -> f64 {
        if self.margin > 0. { GUTTER } else { 0. }
    }
    /// Where the measure starts, from the columns' left edge.
    pub fn text(&self) -> f64 {
        self.margin + self.gutter()
    }
    /// The columns' whole width: margin, measure and any destination column.
    pub fn row(&self) -> f64 {
        self.text() + self.measure + Self::trailing(self.destination)
    }
}
