use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::Path;

use crate::config::{FixtureError, FixtureProfile};
use crate::summary::{FileSummary, FixtureCounts, FixtureSummary, hash_hex, summary_hash};
use recite_core::SourceIdKind;

mod source;

fn append_line(output: &mut String, line: fmt::Arguments<'_>) {
    match output.write_fmt(line) {
        Ok(()) => output.push('\n'),
        Err(_) => unreachable!("writing formatted text to a String cannot fail"),
    }
}

pub fn write_project(
    config: &FixtureProfile,
    output_dir: impl AsRef<Path>,
) -> Result<FixtureSummary, FixtureError> {
    let mut generator = FixtureGenerator::new(config.clone())?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_err(FixtureError::Io)?;
    generator.emit_project(Some(output_dir))
}

pub fn generate_tiny_in_memory(config: &FixtureProfile) -> Result<GeneratedProject, FixtureError> {
    let mut generator = FixtureGenerator::new(config.clone())?;
    generator
        .emit_project(None)
        .map(|summary| GeneratedProject {
            files: generator.files,
            summary,
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedProject {
    pub files: BTreeMap<String, Vec<u8>>,
    pub summary: FixtureSummary,
}

pub(crate) struct FixtureGenerator {
    profile: FixtureProfile,
    files: BTreeMap<String, Vec<u8>>,
}

impl FixtureGenerator {
    pub(crate) fn new(profile: FixtureProfile) -> Result<Self, FixtureError> {
        profile.validate()?;
        Ok(Self {
            profile,
            files: BTreeMap::new(),
        })
    }

    pub(crate) fn emit_project(
        &mut self,
        output_dir: Option<&Path>,
    ) -> Result<FixtureSummary, FixtureError> {
        self.emit_schema();
        self.emit_project_manifest();
        self.emit_runtime_fixture();
        self.emit_locale_catalog();
        self.emit_sources();

        let mut files = Vec::new();
        for (path, content) in &self.files {
            if let Some(output_dir) = output_dir {
                let output_path = output_dir.join(path);
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent).map_err(FixtureError::Io)?;
                }
                fs::write(output_path, content).map_err(FixtureError::Io)?;
            }
            files.push(FileSummary {
                path: path.clone(),
                bytes: content.len() as u64,
                blake3: hash_hex(content),
            });
        }

        let counts = FixtureCounts {
            blocks: self.profile.blocks,
            lines: self.profile.lines,
            choices: self.profile.choices,
            localisable_entries: self.profile.localisable_entries,
            generated_words: self.profile.localisable_entries * self.profile.words_per_entry(),
            shards: self.profile.shards,
        };
        let summary_hash = summary_hash(&self.profile, &counts, &files)?;
        Ok(FixtureSummary {
            profile: self.profile.clone(),
            counts,
            files,
            summary_hash,
        })
    }

    fn insert_text(&mut self, path: impl Into<String>, mut content: String) {
        while content.ends_with("\n\n") {
            content.pop();
        }
        self.files.insert(path.into(), content.into_bytes());
    }

    fn emit_schema(&mut self) {
        let mut speakers = String::new();
        for index in 0..8 {
            append_line(
                &mut speakers,
                format_args!(
                    r#"    "speaker_{index:02}": {{"display_name": "Synthetic Speaker {index:02}"}},"#,
                    index = index,
                ),
            );
        }
        speakers.push_str(r#"    "narrator": {"display_name": "Synthetic Narrator"}"#);

        let schema = [
            "{\n",
            "  \"schema_version\": 1,\n",
            "  \"types\": {\"thread_stage_kind\": {\"kind\": \"enum\", \"values\": [\"fresh\", \"active\", \"resolved\"]}},\n",
            "  \"registries\": {\"thread\": {\"values\": [\"main\", \"side\", \"epilogue\"]}, \"dialogue_sound_effect\": {\"values\": [\"ping\", \"chime\", \"gate\"]}},\n",
            "  \"speakers\": {\n",
            &speakers,
            "\n  },\n",
            "  \"conditions\": {\n",
            "    \"flag\": {\"params\": [{\"name\": \"flag_id\", \"type\": \"string\"}]},\n",
            "    \"counter_gte\": {\"params\": [{\"name\": \"counter_id\", \"type\": \"string\"}, {\"name\": \"threshold\", \"type\": \"int\"}]},\n",
            "    \"relationship\": {\"params\": [{\"name\": \"actor_a\", \"type\": \"speaker\"}, {\"name\": \"actor_b\", \"type\": \"speaker\"}], \"returns\": \"enum:thread_stage_kind\"}\n",
            "  },\n",
            "  \"effects\": {\n",
            "    \"play_sfx\": {\"modes\": [\"immediate\"], \"params\": [{\"name\": \"sound_effect\", \"type\": \"registry:dialogue_sound_effect\"}]},\n",
            "    \"advance_thread\": {\"modes\": [\"deferred\", \"blocking\"], \"params\": [{\"name\": \"thread_id\", \"type\": \"registry:thread\"}, {\"name\": \"stage\", \"type\": \"enum:thread_stage_kind\"}]}\n",
            "  },\n",
            "  \"metadata\": {\n",
            "    \"tier\": {\"targets\": [\"block\"], \"type\": \"string\"},\n",
            "    \"speaker\": {\"targets\": [\"block\", \"line\"], \"type\": \"speaker\"},\n",
            "    \"portrait\": {\"targets\": [\"line\"], \"type\": \"string\"},\n",
            "    \"sfx\": {\"targets\": [\"choice\"], \"type\": \"registry:dialogue_sound_effect\"}\n",
            "  }\n",
            "}\n",
        ]
        .concat();
        self.insert_text("schema/synthetic.schema.json", schema);
    }

    fn emit_project_manifest(&mut self) {
        let mut manifest = String::new();
        append_line(&mut manifest, format_args!("format_version = 1"));
        append_line(&mut manifest, format_args!(""));
        append_line(&mut manifest, format_args!("[project]"));
        append_line(
            &mut manifest,
            format_args!("content_set = \"synthetic-{}\"", self.profile.name),
        );
        append_line(
            &mut manifest,
            format_args!("version = \"{}\"", self.profile.seed),
        );
        append_line(
            &mut manifest,
            format_args!("schema = \"schema/synthetic.schema.json\"\n"),
        );
        append_line(&mut manifest, format_args!("[[scenes]]"));
        append_line(
            &mut manifest,
            format_args!("id = \"synthetic_{}\"", self.profile.name),
        );
        append_line(
            &mut manifest,
            format_args!("asset = \"build/synthetic.recitec\""),
        );
        append_line(&mut manifest, format_args!("block = \"block_00000\""));
        append_line(
            &mut manifest,
            format_args!("participants = [\"speaker_00\"]"),
        );
        self.insert_text("recite.project.toml", manifest);
    }

    fn emit_runtime_fixture(&mut self) {
        let mut fixture = String::new();
        append_line(
            &mut fixture,
            format_args!(
                "[dialogue]\nlocale = \"en-US\"\n[dialogue.catalogs]\nen-US = [\"locales/en-US.po\"]\n"
            ),
        );
        append_line(&mut fixture, format_args!("[conditions]"));
        for index in 0..64 {
            append_line(
                &mut fixture,
                format_args!("\"flag(\\\"flag_{index:02}\\\")\" = true"),
            );
        }
        append_line(
            &mut fixture,
            format_args!("\"counter_gte(\\\"counter_00\\\", 2)\" = true"),
        );
        append_line(
            &mut fixture,
            format_args!("\"relationship(speaker_00, speaker_01)\" = {{ enum = \"active\" }}\n"),
        );
        append_line(
            &mut fixture,
            format_args!(
                "[anchors]\nchoice_anchor_line = \"{}\"\n",
                self.entry_id(SourceIdKind::Line, 0, 0)
            ),
        );
        append_line(&mut fixture, format_args!("[choices]"));
        for block in 0..self.profile.blocks {
            if self.block_has_choices(block) {
                append_line(
                    &mut fixture,
                    format_args!(
                        "\"{}\" = \"{}\"",
                        self.entry_id(SourceIdKind::Line, block, 0),
                        self.entry_id(SourceIdKind::Choice, block, 0)
                    ),
                );
            }
        }
        append_line(
            &mut fixture,
            format_args!("\n[effects]\nauto_ack_blocking = true"),
        );
        self.insert_text("runtime-fixture.toml", fixture);
    }

    fn emit_locale_catalog(&mut self) {
        let mut catalog = String::new();
        self.for_each_entry(|kind, id, text| {
            append_line(&mut catalog, format_args!("msgctxt \"{id}\""));
            append_line(&mut catalog, format_args!("msgid \"{text}\""));
            append_line(
                &mut catalog,
                format_args!("msgstr \"{kind} translation for {id}\"\n"),
            );
        });
        self.insert_text("locales/en-US.po", catalog);
    }
}
