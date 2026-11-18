//! The open catalogue named for display: its language and file.
use super::Localisation;
use recite_writer_session::localisation::setup::languages;

/// The longest language or file name a caption sets.
const NAME_CHARS: usize = 80;

impl Localisation {
    /// The catalogue's language by name ("French"), or its tag when no known language matches.
    pub fn language_name(&self) -> Option<String> {
        let tag = self.slot.document().and_then(|document| {
            document
                .headers()
                .iter()
                .find(|header| header.key().eq_ignore_ascii_case("Language"))
                .map(|header| header.value().to_owned())
        })?;
        let name = languages::name(&tag).unwrap_or(tag);
        (!name.is_empty()).then(|| name.chars().take(NAME_CHARS).collect())
    }
    /// The catalogue language and file name, bounded for display: "French · fr.po".
    pub fn catalogue_caption(&self) -> String {
        let filename: String = self
            .catalogue_path()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "PO catalogue".into())
            .chars()
            .take(NAME_CHARS)
            .collect();
        match self.language_name() {
            Some(language) => format!("{language} · {filename}"),
            None => filename,
        }
    }
}
#[cfg(test)]
mod tests;
