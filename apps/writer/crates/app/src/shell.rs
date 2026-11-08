//! Requests an authoring window raises for its surrounding application shell.

/// Work the shell performs outside the authoring session.
pub enum Request {
    Search(Search),
    Workspace,
}

/// The search surfaces the shell can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Commands,
    GoTo,
    ProjectText,
    Passages,
}
