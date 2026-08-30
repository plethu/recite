use std::{borrow::Borrow, fmt};

/// A normalized project-relative path used to identify one dialogue document.
///
/// `DocumentKey` is deliberately path-neutral. It does not assert that the
/// key exists on disk, identifies a UTF-8 filesystem path, or uses a particular
/// source suffix. Those policies belong to the host that discovers documents.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentKey(String);

impl DocumentKey {
    /// Construct a key after validating normalized project-relative slash
    /// syntax.
    pub fn new(value: impl Into<String>) -> Result<Self, DocumentKeyError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    /// Return the normalized project-relative key without allocating.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for DocumentKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for DocumentKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DocumentKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DocumentKey {
    type Error = DocumentKeyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DocumentKey {
    type Error = DocumentKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

fn validate(value: &str) -> Result<(), DocumentKeyError> {
    if value.is_empty() {
        return Err(DocumentKeyError::Empty);
    }
    if value.starts_with("//") || value.starts_with(r"\\") {
        return Err(DocumentKeyError::Unc {
            value: value.to_owned(),
        });
    }
    if value.starts_with('/') {
        return Err(DocumentKeyError::Absolute {
            value: value.to_owned(),
        });
    }
    if value.len() >= 2 && value.as_bytes()[0].is_ascii_alphabetic() && value.as_bytes()[1] == b':'
    {
        return Err(DocumentKeyError::DrivePrefix {
            value: value.to_owned(),
        });
    }
    if value.contains('\\') {
        return Err(DocumentKeyError::Backslash {
            value: value.to_owned(),
        });
    }

    for (index, component) in value.split('/').enumerate() {
        let error = match component {
            "" => Some(DocumentKeyError::EmptyComponent {
                value: value.to_owned(),
                index,
            }),
            "." => Some(DocumentKeyError::CurrentDirectoryComponent {
                value: value.to_owned(),
                index,
            }),
            ".." => Some(DocumentKeyError::ParentDirectoryComponent {
                value: value.to_owned(),
                index,
            }),
            _ => None,
        };
        if let Some(error) = error {
            return Err(error);
        }
    }

    Ok(())
}

/// Failure to construct a normalized project-relative [`DocumentKey`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentKeyError {
    /// The key contained no path components.
    #[error("document key cannot be empty")]
    Empty,
    /// The key began at a filesystem root.
    #[error("document key must be project-relative: {value:?}")]
    Absolute { value: String },
    /// The key used a Windows UNC path prefix.
    #[error("document key cannot be a UNC path: {value:?}")]
    Unc { value: String },
    /// The key used a Windows drive prefix.
    #[error("document key cannot use a Windows drive prefix: {value:?}")]
    DrivePrefix { value: String },
    /// The key used a platform-native backslash separator.
    #[error("document key must use slash separators: {value:?}")]
    Backslash { value: String },
    /// The key contained two separators or a leading/trailing separator.
    #[error("document key cannot contain an empty component at index {index}: {value:?}")]
    EmptyComponent { value: String, index: usize },
    /// The key contained a current-directory component.
    #[error("document key cannot contain a `.` component at index {index}: {value:?}")]
    CurrentDirectoryComponent { value: String, index: usize },
    /// The key contained a parent-directory component.
    #[error("document key cannot contain a `..` component at index {index}: {value:?}")]
    ParentDirectoryComponent { value: String, index: usize },
}
