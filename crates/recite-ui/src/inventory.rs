use std::fmt;

/// Stable string-backed identity for a shared UI resource.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, ResourceIdError> {
        let value = value.into();
        if value.is_empty()
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
            })
        {
            return Err(ResourceIdError(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceIdError(pub(crate) String);

impl fmt::Display for ResourceIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid resource ID `{}`", self.0)
    }
}

impl std::error::Error for ResourceIdError {}

macro_rules! message_ids {
    ($($variant:ident,)+) => {
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub enum MsgId { $($variant,)+ }

        impl MsgId {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];
            pub const fn key(self) -> &'static str {
                if let Some(key) = cli_message_ids::key(self) { return key; }
                if let Some(key) = tui_message_ids::key(self) { return key; }
                if let Some(key) = lsp_message_ids::key(self) { return key; }
                if let Some(key) = neovim_message_ids::key(self) { return key; }
                panic!("every message ID has one domain key")
            }
            pub fn resource_id(self) -> ResourceId { ResourceId(self.key().to_owned()) }
        }
    };
}

#[path = "inventory/message_ids/cli.rs"]
mod cli_message_ids;
#[path = "inventory/message_ids/lsp.rs"]
mod lsp_message_ids;
#[path = "inventory/message_ids/neovim.rs"]
mod neovim_message_ids;
#[path = "inventory/message_ids/tui.rs"]
mod tui_message_ids;
#[path = "inventory/message_ids/watch.rs"]
mod watch_message_ids;

include!("inventory/message_ids.rs");

pub const ALL_MESSAGE_IDS: &[MsgId] = MsgId::ALL;
pub const MESSAGE_COUNT: usize = MsgId::ALL.len();
