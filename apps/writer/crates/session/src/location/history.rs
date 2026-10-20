//! Accepted workspace locations only; failed preparation never advances the cursor.
use super::Location;
use std::collections::VecDeque;
const LIMIT: usize = 128;

#[derive(Default)]
pub struct History {
    entries: VecDeque<Location>,
    current: usize,
}
pub struct Step {
    from: usize,
    to: usize,
    origin: Location,
    target: Location,
}
impl Step {
    pub fn location(&self) -> &Location {
        &self.target
    }
}
impl History {
    pub fn record(&mut self, location: Location) {
        if self
            .entries
            .get(self.current)
            .is_some_and(|current| current.same_place(&location))
        {
            self.entries[self.current] = location;
            return;
        }
        self.entries.truncate(self.current + 1);
        self.entries.push_back(location);
        if self.entries.len() > LIMIT {
            self.entries.pop_front();
        }
        self.current = self.entries.len() - 1;
    }
    pub fn step(&self, forward: bool) -> Option<Step> {
        let to = if forward {
            self.current.checked_add(1)?
        } else {
            self.current.checked_sub(1)?
        };
        Some(Step {
            from: self.current,
            to,
            origin: self.entries.get(self.current)?.clone(),
            target: self.entries.get(to)?.clone(),
        })
    }
    pub fn accept(&mut self, step: Step, actual: Location) -> bool {
        if self.current != step.from
            || self.entries.get(step.from) != Some(&step.origin)
            || self.entries.get(step.to) != Some(&step.target)
        {
            return false;
        }
        self.current = step.to;
        self.entries[self.current] = actual;
        true
    }
}
