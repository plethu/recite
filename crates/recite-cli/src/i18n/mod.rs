mod locale;
mod messages;

pub(crate) use locale::UiLocale;
pub(crate) use messages::{Messages, MsgId};

#[cfg(test)]
mod tests;
