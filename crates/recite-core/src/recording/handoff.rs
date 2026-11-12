//! One handoff row model with CSV and JSON encodings (spec §9.9).

use serde::{Deserialize, Serialize};

use super::csv::{read_records, write_record};

/// CSV header in column order. Collaborators fill in `take`, `by`, `date` and `note`.
pub const HANDOFF_COLUMNS: [&str; 16] = [
    "id",
    "locale",
    "handoff",
    "block",
    "file",
    "line",
    "speaker",
    "direction",
    "source_text",
    "text",
    "revision",
    "state",
    "take",
    "by",
    "date",
    "note",
];

/// The columns an import cannot do without.
const REQUIRED: [&str; 4] = ["id", "locale", "revision", "take"];

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct HandoffRow {
    pub id: String,
    /// Empty for the source language.
    pub locale: String,
    pub handoff: String,
    pub block: String,
    pub file: String,
    pub line: u32,
    pub speaker: String,
    /// `key=value` pairs in declared direction-key order, separated by `; `.
    pub direction: String,
    pub source_text: String,
    pub text: String,
    pub revision: String,
    pub state: String,
    pub take: String,
    pub by: String,
    pub date: String,
    pub note: String,
}

#[derive(Debug, thiserror::Error)]
pub enum HandoffError {
    #[error("line {line}: {detail}")]
    Csv { line: usize, detail: &'static str },
    #[error("missing required column `{0}`")]
    MissingColumn(&'static str),
    #[error("{0}")]
    Json(String),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HandoffDocument {
    format_version: u32,
    rows: Vec<HandoffRow>,
}

impl HandoffRow {
    fn fields(&self) -> [String; 16] {
        [
            self.id.clone(),
            self.locale.clone(),
            self.handoff.clone(),
            self.block.clone(),
            self.file.clone(),
            self.line.to_string(),
            self.speaker.clone(),
            self.direction.clone(),
            self.source_text.clone(),
            self.text.clone(),
            self.revision.clone(),
            self.state.clone(),
            self.take.clone(),
            self.by.clone(),
            self.date.clone(),
            self.note.clone(),
        ]
    }

    fn set(&mut self, column: &str, value: String) {
        match column {
            "id" => self.id = value,
            "locale" => self.locale = value,
            "handoff" => self.handoff = value,
            "block" => self.block = value,
            "file" => self.file = value,
            "line" => self.line = value.trim().parse().unwrap_or_default(),
            "speaker" => self.speaker = value,
            "direction" => self.direction = value,
            "source_text" => self.source_text = value,
            "text" => self.text = value,
            "revision" => self.revision = value,
            "state" => self.state = value,
            "take" => self.take = value,
            "by" => self.by = value,
            "date" => self.date = value,
            "note" => self.note = value,
            // Spreadsheet users may add their own columns; Recite ignores them.
            _ => {}
        }
    }

    #[must_use]
    pub fn to_csv(rows: &[Self]) -> String {
        let mut output = String::new();
        write_record(&mut output, HANDOFF_COLUMNS);
        for row in rows {
            let fields = row.fields();
            write_record(&mut output, fields.iter().map(String::as_str));
        }
        output
    }

    /// Rows with the source line each starts on, matched to columns by header name.
    pub fn from_csv(input: &str) -> Result<Vec<(usize, Self)>, HandoffError> {
        let mut records = read_records(input)
            .map_err(|(line, detail)| HandoffError::Csv { line, detail })?
            .into_iter();
        let Some((_, header)) = records.next() else {
            return Err(HandoffError::MissingColumn("id"));
        };
        let header = header
            .into_iter()
            .map(|name| name.trim().to_owned())
            .collect::<Vec<_>>();
        if let Some(missing) = REQUIRED
            .iter()
            .find(|name| !header.iter().any(|h| h == *name))
        {
            return Err(HandoffError::MissingColumn(missing));
        }
        Ok(records
            .map(|(line, fields)| {
                let mut row = Self::default();
                for (column, value) in header.iter().zip(fields) {
                    row.set(column, value);
                }
                (line, row)
            })
            .collect())
    }

    #[must_use]
    pub fn to_json(rows: &[Self]) -> String {
        let document = HandoffDocument {
            format_version: 1,
            rows: rows.to_vec(),
        };
        let mut text = serde_json::to_string_pretty(&document)
            .unwrap_or_else(|error| unreachable!("handoff rows always serialize: {error}"));
        text.push('\n');
        text
    }

    /// Rows numbered from 1 in document order.
    pub fn from_json(input: &str) -> Result<Vec<(usize, Self)>, HandoffError> {
        let document: HandoffDocument =
            serde_json::from_str(input).map_err(|error| HandoffError::Json(error.to_string()))?;
        if document.format_version != 1 {
            return Err(HandoffError::Json(format!(
                "unsupported handoff format_version {}",
                document.format_version
            )));
        }
        Ok(document
            .rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| (index + 1, row))
            .collect())
    }
}
