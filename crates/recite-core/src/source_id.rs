use std::fmt;

use crate::{ChoiceId, CoreValueError, LineId};

pub const SOURCE_ID_ANCHOR_HEX_LEN: usize = 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceId {
    Missing,
    Draft { label: String },
    Frozen { label: String, anchor: SourceAnchor },
    Malformed { raw: String },
}

impl SourceId {
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::Missing;
        };
        if raw.matches('@').count() != 1 {
            return Self::Malformed {
                raw: raw.to_owned(),
            };
        }

        let Some((label, anchor)) = raw.split_once('@') else {
            return Self::Malformed {
                raw: raw.to_owned(),
            };
        };
        if !is_valid_source_label(label) {
            return Self::Malformed {
                raw: raw.to_owned(),
            };
        }
        if anchor.is_empty() {
            return Self::Draft {
                label: label.to_owned(),
            };
        }
        let Ok(anchor) = SourceAnchor::new(anchor) else {
            return Self::Malformed {
                raw: raw.to_owned(),
            };
        };
        Self::Frozen {
            label: label.to_owned(),
            anchor,
        }
    }

    #[must_use]
    pub fn frozen(label: impl Into<String>, anchor: SourceAnchor) -> Option<Self> {
        let label = label.into();
        is_valid_source_label(&label).then_some(Self::Frozen { label, anchor })
    }

    #[must_use]
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Draft { label } | Self::Frozen { label, .. } => Some(label),
            Self::Missing | Self::Malformed { .. } => None,
        }
    }

    #[must_use]
    pub fn anchor(&self) -> Option<&SourceAnchor> {
        match self {
            Self::Frozen { anchor, .. } => Some(anchor),
            Self::Missing | Self::Draft { .. } | Self::Malformed { .. } => None,
        }
    }

    #[must_use]
    pub fn display_text(&self) -> Option<String> {
        match self {
            Self::Draft { label } => Some(format!("{label}@")),
            Self::Frozen { label, anchor } => Some(format!("{label}@{anchor}")),
            Self::Malformed { raw } => Some(raw.clone()),
            Self::Missing => None,
        }
    }

    #[must_use]
    pub fn generated_anchor(
        project_relative_path: &str,
        kind: SourceIdKind,
        line: u32,
        column: u32,
        label: &str,
        salt: u32,
    ) -> SourceAnchor {
        let mut hasher = blake3::Hasher::new();
        hasher.update(project_relative_path.as_bytes());
        hasher.update(&[0]);
        hasher.update(kind.as_str().as_bytes());
        hasher.update(&[0]);
        hasher.update(&line.to_le_bytes());
        hasher.update(&column.to_le_bytes());
        hasher.update(&[0]);
        hasher.update(label.as_bytes());
        hasher.update(&[0]);
        hasher.update(&salt.to_le_bytes());
        let digest = hasher.finalize();
        SourceAnchor::from_first_80_bits(digest.as_bytes())
    }

    #[must_use]
    pub fn canonical_line_id(&self) -> Option<LineId> {
        self.anchor()
            .and_then(|anchor| LineId::new(anchor.as_str()).ok())
    }

    #[must_use]
    pub fn canonical_choice_id(&self) -> Option<ChoiceId> {
        self.anchor()
            .and_then(|anchor| ChoiceId::new(anchor.as_str()).ok())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SourceIdKind {
    Line,
    Choice,
}

impl SourceIdKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Choice => "choice",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SourceAnchor(String);

impl SourceAnchor {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreValueError> {
        let value = value.into();
        if is_valid_source_anchor(&value) {
            Ok(Self(value))
        } else {
            Err(CoreValueError::InvalidValue {
                kind: "SourceAnchor",
                value,
            })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_first_80_bits(bytes: &[u8; blake3::OUT_LEN]) -> Self {
        let mut anchor = String::with_capacity(SOURCE_ID_ANCHOR_HEX_LEN);
        for byte in &bytes[..10] {
            use fmt::Write as _;
            let _ = write!(&mut anchor, "{byte:02x}");
        }
        Self(anchor)
    }
}

impl fmt::Display for SourceAnchor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[must_use]
pub fn is_valid_source_anchor(value: &str) -> bool {
    value.len() == SOURCE_ID_ANCHOR_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[must_use]
pub fn is_valid_source_label(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (unicode_ident::is_xid_start(first) || first == '_')
        && chars.all(|character| {
            unicode_ident::is_xid_continue(character) || matches!(character, '_' | '-' | '.')
        })
}
