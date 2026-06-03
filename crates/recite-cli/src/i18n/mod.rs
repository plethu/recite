mod locale;
mod messages;

pub(crate) use locale::UiLocale;
pub(crate) use messages::{Messages, MsgId};

#[cfg(test)]
pub(crate) use locale::DEFAULT_LOCALE;
#[cfg(test)]
pub(crate) use messages::DEFAULT_RESOURCE;

#[cfg(test)]
mod tests;
