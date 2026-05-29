// Fixturegen is a deterministic generator/tooling crate; string-write panics indicate logic bugs.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::config::{FixtureError, FixtureProfile};
use crate::content::GeneratedText;
use crate::summary::{FileSummary, FixtureCounts, FixtureSummary, hash_hex, summary_hash};

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
            writeln!(
                &mut speakers,
                r#"    "speaker_{index:02}": {{"display_name": "Synthetic Speaker {index:02}"}},"#
            )
            .expect("write string");
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
        writeln!(&mut manifest, "[project]").expect("write string");
        writeln!(
            &mut manifest,
            "content_set = \"synthetic-{}\"",
            self.profile.name
        )
        .expect("write string");
        writeln!(&mut manifest, "version = \"{}\"", self.profile.seed).expect("write string");
        writeln!(&mut manifest, "schema = \"schema/synthetic.schema.json\"\n")
            .expect("write string");
        writeln!(&mut manifest, "[[scenes]]").expect("write string");
        writeln!(&mut manifest, "id = \"synthetic_{}\"", self.profile.name).expect("write string");
        writeln!(&mut manifest, "asset = \"build/synthetic.recitec\"").expect("write string");
        writeln!(&mut manifest, "block = \"block_00000\"").expect("write string");
        writeln!(
            &mut manifest,
            "participants = [\"narrator\", \"speaker_00\"]"
        )
        .expect("write string");
        self.insert_text("recite.project.toml", manifest);
    }

    fn emit_runtime_fixture(&mut self) {
        let mut fixture = String::new();
        writeln!(
            &mut fixture,
            "[dialogue]\nlocale = \"en-US\"\n[dialogue.catalogs]\nen-US = [\"locales/en-US.po\"]\n"
        )
        .expect("write string");
        writeln!(&mut fixture, "[conditions]").expect("write string");
        for index in 0..64 {
            writeln!(&mut fixture, "\"flag(\\\"flag_{index:02}\\\")\" = true")
                .expect("write string");
        }
        writeln!(
            &mut fixture,
            "\"counter_gte(\\\"counter_00\\\", 2)\" = true"
        )
        .expect("write string");
        writeln!(
            &mut fixture,
            "\"relationship(speaker_00, speaker_01)\" = {{ enum = \"active\" }}\n"
        )
        .expect("write string");
        writeln!(&mut fixture, "[choices]").expect("write string");
        for block in 0..self.profile.blocks {
            writeln!(
                &mut fixture,
                "line_{block:05}_000 = \"choice_{block:05}_000\""
            )
            .expect("write string");
        }
        writeln!(&mut fixture, "\n[effects]\nauto_ack_blocking = true").expect("write string");
        self.insert_text("runtime-fixture.toml", fixture);
    }

    fn emit_locale_catalog(&mut self) {
        let mut catalog = String::new();
        self.for_each_entry(|kind, id, text| {
            writeln!(&mut catalog, "msgctxt \"{id}\"").expect("write string");
            writeln!(&mut catalog, "msgid \"{text}\"").expect("write string");
            writeln!(&mut catalog, "msgstr \"{kind} translation for {id}\"\n")
                .expect("write string");
        });
        self.insert_text("locales/en-US.po", catalog);
    }

    fn emit_sources(&mut self) {
        let blocks_per_shard = self.profile.blocks.div_ceil(self.profile.shards);
        for shard in 0..self.profile.shards {
            let start = shard * blocks_per_shard;
            let end = ((shard + 1) * blocks_per_shard).min(self.profile.blocks);
            if start >= end {
                continue;
            }
            let mut source = String::new();
            for block in start..end {
                self.emit_block(block, &mut source);
            }
            self.insert_text(format!("src/shard-{shard:03}.recite"), source);
        }
    }

    fn emit_block(&mut self, block: u32, source: &mut String) {
        let default = if block == 0 { " default" } else { "" };
        let speaker = block % 8;
        writeln!(
            source,
            ":: block_{block:05}{default} speaker=speaker_{speaker:02} tier=\"{}\"",
            self.profile.name
        )
        .expect("write string");
        writeln!(source, "# synthetic block {block:05}").expect("write string");
        if block.is_multiple_of(5) {
            writeln!(source, "! immediate play_sfx(ping)").expect("write string");
        }
        if block.is_multiple_of(7) {
            writeln!(source, "! deferred advance_thread(main, active)").expect("write string");
        }
        if block == 0 {
            writeln!(source, "! blocking advance_thread(main, active)").expect("write string");
        }

        let lines = self.lines_in_block(block);
        for line in 0..lines {
            if line == 0 && self.uses_relationship_match(block) {
                writeln!(source, ":match relationship(speaker_00, speaker_01)")
                    .expect("write string");
                writeln!(source, "  :case active").expect("write string");
                self.emit_line(block, line, speaker, 4, source);
            } else {
                self.emit_line(block, line, speaker, 0, source);
            }
        }
        writeln!(source, "-> END\n").expect("write string");
    }

    fn emit_line(&self, block: u32, line: u32, speaker: u32, indent: usize, source: &mut String) {
        let prefix = " ".repeat(indent);
        let body_prefix = " ".repeat(indent + 2);
        writeln!(
            source,
            "{prefix}> line_{block:05}_{line:03} speaker=speaker_{speaker:02} portrait=\"portrait_{speaker:02}\""
        )
        .expect("write string");
        writeln!(
            source,
            "{body_prefix}{}",
            self.entry_text("line", block, line)
        )
        .expect("write string");
        if line == 0 {
            self.emit_choices(block, indent + 2, source);
        }
    }

    fn emit_choices(&self, block: u32, indent: usize, source: &mut String) {
        let choice_prefix = " ".repeat(indent);
        let body_prefix = " ".repeat(indent + 2);
        for choice in 0..self.choices_in_block(block) {
            let target = self.choice_target(block, choice);
            let condition = if choice % 2 == 0 {
                format!(" if flag(\"flag_{:02}\")", block % 64)
            } else {
                " if counter_gte(\"counter_00\", 2)".to_owned()
            };
            writeln!(
                source,
                "{choice_prefix}? choice_{block:05}_{choice:03} sfx=chime{condition}"
            )
            .expect("write string");
            writeln!(
                source,
                "{body_prefix}{}",
                self.entry_text("choice", block, choice)
            )
            .expect("write string");
            writeln!(source, "{body_prefix}-> {target}").expect("write string");
        }
    }

    fn lines_in_block(&self, block: u32) -> u32 {
        distributed_count(self.profile.lines, self.profile.blocks, block)
    }

    fn choices_in_block(&self, block: u32) -> u32 {
        distributed_count(self.profile.choices, self.profile.blocks, block)
    }

    fn uses_relationship_match(&self, block: u32) -> bool {
        block.is_multiple_of(3)
    }

    fn choice_target(&self, block: u32, choice: u32) -> String {
        let offset = if choice == 0 { 1 } else { (choice % 3) + 1 };
        let Some(target_block) = block.checked_add(offset) else {
            return "END".to_owned();
        };
        if target_block < self.profile.blocks {
            self.reference_to_block(block, target_block)
        } else {
            "END".to_owned()
        }
    }

    fn reference_to_block(&self, source_block: u32, target_block: u32) -> String {
        let source_shard = self.shard_for_block(source_block);
        let target_shard = self.shard_for_block(target_block);
        if source_shard == target_shard {
            format!("block_{target_block:05}")
        } else {
            format!("src/shard-{target_shard:03}.recite::block_{target_block:05}")
        }
    }

    fn shard_for_block(&self, block: u32) -> u32 {
        let blocks_per_shard = self.profile.blocks.div_ceil(self.profile.shards);
        block / blocks_per_shard
    }

    fn entry_text(&self, kind: &str, block: u32, index: u32) -> String {
        GeneratedText::new(self.profile.seed, self.profile.words_per_entry())
            .entry(kind, block, index)
    }

    fn for_each_entry(&self, mut emit: impl FnMut(&str, String, String)) {
        for block in 0..self.profile.blocks {
            for line in 0..self.lines_in_block(block) {
                emit(
                    "line",
                    format!("line_{block:05}_{line:03}"),
                    self.entry_text("line", block, line),
                );
                if line == 0 {
                    for choice in 0..self.choices_in_block(block) {
                        emit(
                            "choice",
                            format!("choice_{block:05}_{choice:03}"),
                            self.entry_text("choice", block, choice),
                        );
                    }
                }
            }
        }
    }
}

fn distributed_count(total: u32, buckets: u32, index: u32) -> u32 {
    let base = total / buckets;
    let remainder = total % buckets;
    base + u32::from(index < remainder)
}
