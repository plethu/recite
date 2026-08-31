use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InputFormat {
    StandaloneToml,
    GeneratedJson,
}

impl InputFormat {
    pub(super) fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("toml") => Some(Self::StandaloneToml),
            Some("json") => Some(Self::GeneratedJson),
            _ => None,
        }
    }

    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::StandaloneToml => "standalone_toml",
            Self::GeneratedJson => "generated_json",
        }
    }
}
