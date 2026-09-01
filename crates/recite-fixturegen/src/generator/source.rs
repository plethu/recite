use recite_core::{SourceId, SourceIdKind};

use super::{FixtureGenerator, append_line};
use crate::content::GeneratedText;

const END_TARGET: &str = recite_core::END_DIVERT_TARGET;

impl FixtureGenerator {
    pub(super) fn emit_sources(&mut self) {
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
        append_line(
            source,
            format_args!(
                ":: block_{block:05}{default} speaker=speaker_{speaker:02} tier=\"{}\"",
                self.profile.name
            ),
        );
        append_line(source, format_args!("# synthetic block {block:05}"));
        if block.is_multiple_of(5) {
            append_line(source, format_args!("! immediate play_sfx(ping)"));
        }
        if block.is_multiple_of(7) {
            append_line(
                source,
                format_args!("! deferred advance_thread(main, active)"),
            );
        }
        if block == 0 {
            append_line(
                source,
                format_args!("! blocking advance_thread(main, active)"),
            );
        }
        let lines = self.lines_in_block(block);
        for line in 0..lines {
            if line == 0 && self.uses_relationship_match(block) {
                append_line(
                    source,
                    format_args!(":match relationship(speaker_00, speaker_01)"),
                );
                append_line(source, format_args!("  :case active"));
                self.emit_line(block, line, speaker, 4, source);
            } else {
                self.emit_line(block, line, speaker, 0, source);
            }
        }
        append_line(
            source,
            format_args!("-> {}\n", self.block_fallthrough_target(block)),
        );
    }
    fn emit_line(&self, block: u32, line: u32, speaker: u32, indent: usize, source: &mut String) {
        let prefix = " ".repeat(indent);
        let body_prefix = " ".repeat(indent + 2);
        append_line(
            source,
            format_args!(
                "{prefix}> {} speaker=speaker_{speaker:02} portrait=\"portrait_{speaker:02}\"",
                self.entry_source_id(SourceIdKind::Line, block, line)
            ),
        );
        append_line(
            source,
            format_args!("{body_prefix}{}", self.entry_text("line", block, line)),
        );
        if line == 0 && self.block_has_choices(block) {
            self.emit_choices(block, indent + 2, source);
        }
    }
    fn emit_choices(&self, block: u32, indent: usize, source: &mut String) {
        let choice_prefix = " ".repeat(indent);
        let body_prefix = " ".repeat(indent + 2);
        for choice in 0..self.choices_in_block(block) {
            let target = self.choice_target(block, choice);
            let condition = if self.choice_conditions_enabled(block) {
                if choice % 2 == 0 {
                    format!(" requires=(flag(\"flag_{:02}\"))", block % 64)
                } else {
                    " requires=(counter_gte(\"counter_00\", 2))".to_owned()
                }
            } else {
                String::new()
            };
            append_line(
                source,
                format_args!(
                    "{choice_prefix}? {} sfx=chime{condition}",
                    self.entry_source_id(SourceIdKind::Choice, block, choice)
                ),
            );
            append_line(
                source,
                format_args!("{body_prefix}{}", self.entry_text("choice", block, choice)),
            );
            append_line(source, format_args!("{body_prefix}-> {target}"));
        }
    }
    pub(super) fn lines_in_block(&self, block: u32) -> u32 {
        distributed_count(self.profile.lines, self.profile.blocks, block)
    }

    fn choices_in_block(&self, block: u32) -> u32 {
        if !self.block_has_choices(block) {
            return 0;
        }

        distributed_count(
            self.profile.choices,
            self.choice_block_count(),
            self.choice_block_index(block),
        )
    }

    pub(super) fn block_has_choices(&self, block: u32) -> bool {
        block.is_multiple_of(2)
    }

    fn choice_block_count(&self) -> u32 {
        self.profile.blocks.div_ceil(2)
    }

    fn choice_block_index(&self, block: u32) -> u32 {
        block / 2
    }

    fn choice_conditions_enabled(&self, block: u32) -> bool {
        block.is_multiple_of(4)
    }

    fn uses_relationship_match(&self, block: u32) -> bool {
        block.is_multiple_of(3)
    }

    fn choice_target(&self, block: u32, choice: u32) -> String {
        let offset = if choice == 0 { 1 } else { (choice % 3) + 1 };
        let Some(target_block) = block.checked_add(offset) else {
            return END_TARGET.to_owned();
        };
        if target_block < self.profile.blocks {
            self.reference_to_block(block, target_block)
        } else {
            END_TARGET.to_owned()
        }
    }

    fn block_fallthrough_target(&self, block: u32) -> String {
        if self.block_has_choices(block) {
            return END_TARGET.to_owned();
        }

        let Some(target_block) = block.checked_add(1) else {
            return END_TARGET.to_owned();
        };
        if target_block < self.profile.blocks {
            self.reference_to_block(block, target_block)
        } else {
            END_TARGET.to_owned()
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

    pub(super) fn for_each_entry(&self, mut emit: impl FnMut(&str, String, String)) {
        for block in 0..self.profile.blocks {
            for line in 0..self.lines_in_block(block) {
                emit(
                    "line",
                    self.entry_id(SourceIdKind::Line, block, line),
                    self.entry_text("line", block, line),
                );
                if line == 0 && self.block_has_choices(block) {
                    for choice in 0..self.choices_in_block(block) {
                        emit(
                            "choice",
                            self.entry_id(SourceIdKind::Choice, block, choice),
                            self.entry_text("choice", block, choice),
                        );
                    }
                }
            }
        }
    }

    fn entry_source_id(&self, kind: SourceIdKind, block: u32, index: u32) -> String {
        let label = self.entry_label(kind, block, index);
        format!("{label}@{}", self.entry_id(kind, block, index))
    }

    pub(super) fn entry_id(&self, kind: SourceIdKind, block: u32, index: u32) -> String {
        let label = self.entry_label(kind, block, index);
        let path = format!("src/shard-{:03}.recite", self.shard_for_block(block));
        SourceId::generated_anchor(
            &path,
            kind,
            block.saturating_add(1),
            index.saturating_add(1),
            &label,
            0,
        )
        .to_string()
    }

    fn entry_label(&self, kind: SourceIdKind, block: u32, index: u32) -> String {
        match kind {
            SourceIdKind::Line => format!("line_{block:05}_{index:03}"),
            SourceIdKind::Choice => format!("choice_{block:05}_{index:03}"),
        }
    }
}

fn distributed_count(total: u32, buckets: u32, index: u32) -> u32 {
    let base = total / buckets;
    let remainder = total % buckets;
    base + u32::from(index < remainder)
}
