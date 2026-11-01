//! Exact navigation chords shared by native hosts; writing fields retain ordinary text keys.
use recite_config::WriterCommand;

#[derive(Clone, Copy)]
pub enum Chord<'a> {
    Plain(&'a str),
    Control(&'a str),
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Command(WriterCommand),
    PanePrefix,
}
pub fn navigation(chord: Chord<'_>, pending: bool) -> Option<Action> {
    if pending {
        let (Chord::Plain(key) | Chord::Control(key)) = chord else {
            return None;
        };
        return Some(Action::Command(match key {
            "h" => WriterCommand::PaneLeft,
            "j" => WriterCommand::PaneDown,
            "k" => WriterCommand::PaneUp,
            "l" => WriterCommand::PaneRight,
            _ => return None,
        }));
    }
    match chord {
        Chord::Control(key) if key.eq_ignore_ascii_case("o") => {
            Some(Action::Command(WriterCommand::Back))
        }
        Chord::Control(key) if key.eq_ignore_ascii_case("i") => {
            Some(Action::Command(WriterCommand::Forward))
        }
        Chord::Control(key) if key.eq_ignore_ascii_case("w") => Some(Action::PanePrefix),
        Chord::Plain(key) => Some(Action::Command(match key {
            ":" => WriterCommand::Commands,
            "/" => WriterCommand::Find,
            "n" => WriterCommand::NextMatch,
            "N" => WriterCommand::PreviousMatch,
            _ => return None,
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
