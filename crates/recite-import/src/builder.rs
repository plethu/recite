use std::collections::BTreeSet;

use recite_compiler::compile::{
    CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema,
};
use recite_core::compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use recite_core::{
    DiagnosticCode, SourceAnchor, SourceId, SourcePosition, SourceSpan, SpeakerId,
    schema::ProjectSchema,
};

use crate::diagnostics::{INVALID, LOSS, UNSUPPORTED, item};
use crate::{
    Action, ImportError, ImportReport, ImportStatus, Location, Provenance, SourceFamily,
    SourceMapping,
};

pub(super) struct Builder {
    report: ImportReport,
    blocks: BTreeSet<String>,
    block: Option<String>,
    terminal: bool,
    prompt: bool,
    source_lines: usize,
}

pub(super) enum Target<'a> {
    Block(&'a str),
    End,
}

impl Target<'_> {
    fn source(&self) -> String {
        match self {
            Self::Block(name) => block_name(name),
            Self::End => "END".to_owned(),
        }
    }
}

impl Builder {
    pub(super) fn new(family: SourceFamily, file: &str) -> Self {
        Self {
            report: ImportReport {
                format_version: 1,
                family,
                file: file.to_owned(),
                status: ImportStatus::Invalid,
                source: String::new(),
                mappings: Vec::new(),
                items: Vec::new(),
                native_diagnostics: Vec::new(),
            },
            blocks: BTreeSet::new(),
            block: None,
            terminal: false,
            prompt: false,
            source_lines: 0,
        }
    }

    pub(super) fn provenance(&self, location: Location, key: impl Into<String>) -> Provenance {
        Provenance {
            file: self.report.file.clone(),
            location,
            record_key: key.into(),
        }
    }

    pub(super) fn text_provenance(
        &self,
        line: usize,
        text: &str,
    ) -> Result<Provenance, ImportError> {
        let line = u32::try_from(line).map_err(|_| ImportError::PositionOverflow)?;
        let end =
            u32::try_from(text.chars().count() + 1).map_err(|_| ImportError::PositionOverflow)?;
        Ok(self.provenance(
            Location::Text {
                span: SourceSpan::new(
                    &self.report.file,
                    SourcePosition::new(line, 1)?,
                    Some(SourcePosition::new(line, end)?),
                ),
            },
            line.to_string(),
        ))
    }

    pub(super) fn issue(
        &mut self,
        code: DiagnosticCode,
        provenance: Provenance,
        construct: &str,
        detail: impl Into<String>,
    ) -> Result<(), ImportError> {
        self.report
            .items
            .push(item(code, provenance, construct, detail)?);
        Ok(())
    }

    pub(super) fn block(&mut self, name: &str, provenance: Provenance) -> Result<(), ImportError> {
        self.close_block();
        self.block = None;
        self.prompt = false;
        self.terminal = false;
        if name.is_empty() || !self.blocks.insert(name.to_owned()) {
            return self.issue(
                INVALID,
                provenance,
                "block",
                format!("Empty or repeated block: {name}"),
            );
        }
        let target = block_name(name);
        let default = if self.blocks.len() == 1 {
            " default"
        } else {
            ""
        };
        self.record(&provenance, "block", Some(name), &target);
        self.append(&format!(":: {target}{default}\n"));
        self.block = Some(name.to_owned());
        Ok(())
    }

    pub(super) fn current_block(&self) -> Option<&str> {
        self.block.as_deref()
    }

    pub(super) fn line(
        &mut self,
        text: &str,
        speaker: Option<&str>,
        id: Option<&str>,
        provenance: Provenance,
    ) -> Result<(), ImportError> {
        if !self.accept_text(text, &provenance)? {
            return Ok(());
        }
        if speaker.is_some_and(|value| !speaker_value(value)) {
            return self.issue(
                INVALID,
                provenance,
                "speaker",
                "Speaker must be a nonempty ID that fits one bare native header value.",
            );
        }
        if self.block.is_none() || self.terminal {
            return self.issue(
                UNSUPPORTED,
                provenance,
                "line",
                "Text outside a block or after a terminal jump/choice group.",
            );
        }
        let stable = self.stable_id(id, &provenance);
        let speaker = speaker.map_or(String::new(), |value| format!(" speaker={value}"));
        self.record(&provenance, "line", id, &stable);
        self.append(&format!("> {stable}{speaker}\n  {text}\n"));
        self.prompt = true;
        Ok(())
    }

    pub(super) fn choice(
        &mut self,
        text: &str,
        target: Target<'_>,
        provenance: Provenance,
    ) -> Result<(), ImportError> {
        if !self.accept_text(text, &provenance)? {
            return Ok(());
        }
        if !self.prompt {
            return self.issue(
                UNSUPPORTED,
                provenance,
                "choice",
                "Choice needs a preceding dialogue line and a static target.",
            );
        }
        let stable = self.stable_id(None, &provenance);
        self.record(&provenance, "choice", None, &stable);
        self.append(&format!(
            "  ? {stable}\n    {text}\n    -> {}\n",
            target.source()
        ));
        self.terminal = true;
        Ok(())
    }

    pub(super) fn jump(
        &mut self,
        target: Target<'_>,
        provenance: Provenance,
    ) -> Result<(), ImportError> {
        if self.block.is_none() || self.terminal {
            return self.issue(
                UNSUPPORTED,
                provenance,
                "jump",
                "Jump outside a block, after a terminal statement, or without a target.",
            );
        }
        let target = target.source();
        self.record(&provenance, "jump", None, &target);
        self.append(&format!("-> {target}\n"));
        self.terminal = true;
        self.prompt = false;
        Ok(())
    }

    fn accept_text(&mut self, text: &str, provenance: &Provenance) -> Result<bool, ImportError> {
        if text.is_empty()
            || text.contains(['\n', '\r', '\t', '\\', '{', '}', '[', ']'])
            || text.trim_start().starts_with(['>', '?', '!', ':', '#'])
            || text.trim_start().starts_with("->")
            || text.trim_start().starts_with("//")
        {
            self.issue(UNSUPPORTED, provenance.clone(), "text", "Only nonempty single-line plain text is supported; migrate markup, interpolation and escapes manually.")?;
            return Ok(false);
        }
        if text.trim() != text {
            self.issue(
                LOSS,
                provenance.clone(),
                "whitespace",
                "Leading or trailing whitespace may be normalized by native source parsing.",
            )?;
        }
        Ok(true)
    }

    fn stable_id(&self, original: Option<&str>, provenance: &Provenance) -> String {
        if let Some(id) = original {
            if matches!(SourceId::parse(Some(id)), SourceId::Frozen { .. }) {
                return id.to_owned();
            }
            if SourceAnchor::new(id).is_ok() {
                return format!("imported_{id}@{id}");
            }
        }
        let mut hasher = blake3::Hasher::new();
        for value in [
            &self.report.file,
            &provenance.record_key,
            original.unwrap_or(""),
        ] {
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }
        let anchor = &hasher.finalize().to_hex()[..20];
        format!("imported_{anchor}@{anchor}")
    }

    fn record(
        &mut self,
        provenance: &Provenance,
        construct: &str,
        original: Option<&str>,
        generated: &str,
    ) {
        self.report.mappings.push(SourceMapping {
            provenance: provenance.clone(),
            construct: construct.to_owned(),
            original_id: original.map(str::to_owned),
            generated_id: generated.to_owned(),
            generated_line: self.source_lines + 1,
        });
    }

    fn append(&mut self, source: &str) {
        self.source_lines += source.bytes().filter(|byte| *byte == b'\n').count();
        self.report.source.push_str(source);
    }

    fn close_block(&mut self) {
        if self.block.is_some() && !self.terminal {
            self.append("-> END\n");
        }
    }

    pub(super) fn finish(
        mut self,
        schema: Option<&ProjectSchema>,
    ) -> Result<ImportReport, ImportError> {
        self.close_block();
        if self.blocks.is_empty() {
            self.issue(
                INVALID,
                self.provenance(Location::Document, "document"),
                "document",
                "No supported dialogue blocks were found.",
            )?;
        }
        let options = CompileOptions::new(
            CompilerVersion::new(env!("CARGO_PKG_VERSION"))?,
            CompiledAssetId::new("migration")?,
            SourceMapId::new("migration.map")?,
            schema.map_or(
                SchemaFingerprint::NoSchema,
                ProjectSchema::canonical_fingerprint,
            ),
        );
        let inputs = [CompileInput::new("imported.recite", &self.report.source)];
        let compiled = if let Some(schema) = schema {
            compile_inputs_with_schema(inputs, options, schema)?
        } else {
            compile_inputs(inputs, options)?
        };
        self.report.native_diagnostics = compiled
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.record())
            .collect::<Result<_, _>>()?;
        self.report.status = if compiled.asset.is_none()
            || self.blocks.is_empty()
            || self
                .report
                .items
                .iter()
                .any(|item| item.action == Action::Rejected)
        {
            ImportStatus::Invalid
        } else if self.report.items.is_empty() {
            ImportStatus::Complete
        } else {
            ImportStatus::Partial
        };
        Ok(self.report)
    }
}

pub(super) fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) fn speaker_value(value: &str) -> bool {
    // SpeakerId permits Unicode. Keep the ID verbatim in one bare header field;
    // quoting and bracket/parenthesis grouping belong to native header syntax.
    SpeakerId::new(value).is_ok()
        && !value.chars().any(|character| {
            character.is_whitespace()
                || character.is_control()
                || matches!(character, '"' | '\\' | '[' | ']' | '(' | ')')
        })
}

pub(super) fn block_name(name: &str) -> String {
    // Always namespace generated blocks: an authored name cannot collide with
    // a generated fallback or Recite's END target.
    format!("block_{}", &blake3::hash(name.as_bytes()).to_hex()[..20])
}
