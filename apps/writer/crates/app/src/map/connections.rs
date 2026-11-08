//! Accepted incident-link indices and their independent bounded list pages.
use super::*;
#[derive(Clone, Copy)]
pub enum Direction {
    Outgoing,
    Incoming,
}
impl Direction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Outgoing => "Outgoing",
            Self::Incoming => "Incoming",
        }
    }
    pub fn target(self, link: &recite_writer_model::SceneLink) -> &str {
        match self {
            Self::Outgoing => &link.destination,
            Self::Incoming => &link.origin,
        }
    }
}
#[derive(Default)]
pub struct List {
    pub indices: Vec<usize>,
    pub page: usize,
}
#[derive(Default)]
pub struct Connections {
    pub expanded: bool,
    outgoing: List,
    incoming: List,
}
impl Connections {
    pub fn refresh(&mut self, graph: &Topology, selected: Option<&str>) {
        let next = Self::for_selection(graph, selected);
        if self.outgoing.indices != next.outgoing.indices {
            self.outgoing = next.outgoing;
        }
        if self.incoming.indices != next.incoming.indices {
            self.incoming = next.incoming;
        }
    }
    pub fn for_selection(graph: &Topology, selected: Option<&str>) -> Self {
        let mut lists = Self::default();
        if let Some(selected) = selected
            && let Some(index) = graph.ids.get(selected)
        {
            for index in &graph.incident[*index] {
                let link = &graph.links[*index];
                if link.origin == selected {
                    lists.outgoing.indices.push(*index);
                } else if link.destination == selected {
                    lists.incoming.indices.push(*index);
                }
            }
        }
        lists
    }
    pub fn list(&self, direction: Direction) -> &List {
        match direction {
            Direction::Outgoing => &self.outgoing,
            Direction::Incoming => &self.incoming,
        }
    }
    pub fn list_mut(&mut self, direction: Direction) -> &mut List {
        match direction {
            Direction::Outgoing => &mut self.outgoing,
            Direction::Incoming => &mut self.incoming,
        }
    }
}

#[cfg(test)]
mod tests;
