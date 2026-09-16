//! Geometric routes have independent ports and lanes; they never classify story flow.
use super::layout::{HEIGHT, Node};
use recite_writer_model::{SceneLink, ScriptBlock};

#[derive(Clone, PartialEq)]
pub(super) struct Route {
    pub path: String,
    pub arrow: String,
    pub bounds: (f32, f32, f32, f32),
    /// Anchor on the connection, in world coordinates.
    pub label: (f32, f32),
}
pub(super) fn routes(
    blocks: &[ScriptBlock],
    nodes: &[Node],
    links: &[SceneLink],
) -> Vec<Option<Route>> {
    let right = nodes.iter().map(|n| n.x + n.width).fold(0., f32::max) + 24.;
    let mut lane = 0;
    links
        .iter()
        .enumerate()
        .map(|(index, link)| {
            let from = &nodes[blocks.iter().position(|b| b.id == link.origin)?];
            let to = &nodes[blocks.iter().position(|b| b.id == link.destination)?];
            let outgoing = links.iter().filter(|l| l.origin == link.origin).count();
            let out_slot = links[..index]
                .iter()
                .filter(|l| l.origin == link.origin)
                .count();
            let incoming = links
                .iter()
                .filter(|l| l.destination == link.destination)
                .count();
            let in_slot = links[..index]
                .iter()
                .filter(|l| l.destination == link.destination)
                .count();
            let x1 = from.x + from.width * (out_slot + 1) as f32 / (outgoing + 1) as f32;
            let y1 = from.y + HEIGHT;
            let x2 = to.x + to.width * (in_slot + 1) as f32 / (incoming + 1) as f32;
            let y2 = to.y;
            let side = link.returning || y2 <= y1 || y2 - y1 > 120.;
            let rail = if side {
                let x = right + lane as f32 * 24.;
                lane += 1;
                x
            } else {
                x1
            };
            let lead = 16. + out_slot as f32 * 8.;
            let approach = 16. + in_slot as f32 * 8.;
            let path = if side {
                format!(
                    "M{x1} {y1} L{x1} {} L{rail} {} L{rail} {} L{x2} {} L{x2} {y2}",
                    y1 + lead,
                    y1 + lead,
                    y2 - approach,
                    y2 - approach
                )
            } else {
                format!(
                    "M{x1} {y1} C{x1} {} {x2} {} {x2} {y2}",
                    (y1 + y2) / 2.,
                    (y1 + y2) / 2.
                )
            };
            let left = x1.min(x2) - 5.;
            let top = y1.min(y2) - approach - 5.;
            let width = rail.max(x1).max(x2) - left + 5.;
            let height = y1.max(y2) + lead - top + 5.;
            Some(Route {
                path,
                arrow: format!(
                    "M{} {} L{x2} {y2} L{} {}",
                    x2 - 3.,
                    y2 - 5.,
                    x2 + 3.,
                    y2 - 5.
                ),
                bounds: (left, top, width, height),
                label: if side {
                    (rail, (y1 + lead + y2 - approach) / 2.)
                } else {
                    // For these symmetric cubic controls, t=0.5 is the endpoint midpoint.
                    ((x1 + x2) / 2., (y1 + y2) / 2.)
                },
            })
        })
        .collect()
}
#[cfg(test)]
mod tests;
