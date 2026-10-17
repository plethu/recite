use crate::{Passage, ScriptBlock, ScriptEntry};
use recite_parser::SourceRegion;

pub(super) struct Relocation {
    old_byte: usize,
    new_byte: usize,
    old_line: u32,
    new_line: u32,
}

impl Relocation {
    pub(super) fn origin(region: &SourceRegion<'_>) -> Self {
        Self {
            old_byte: 0,
            new_byte: region.byte_range().start,
            old_line: 1,
            new_line: region.first_line(),
        }
    }

    pub(super) fn after(
        old: &SourceRegion<'_>,
        new: &SourceRegion<'_>,
        old_next: Option<&SourceRegion<'_>>,
        new_next: Option<&SourceRegion<'_>>,
    ) -> Option<Self> {
        let (old_line, new_line) = match (old_next, new_next) {
            (Some(old), Some(new)) => (old.first_line(), new.first_line()),
            (None, None) => (old.first_line(), new.first_line()),
            _ => return None,
        };
        Some(Self {
            old_byte: old.byte_range().end,
            new_byte: new.byte_range().end,
            old_line,
            new_line,
        })
    }

    pub(super) fn passage(&self, passage: &mut Passage) -> Option<()> {
        passage.text_range.start = passage
            .text_range
            .start
            .checked_sub(self.old_byte)?
            .checked_add(self.new_byte)?;
        passage.text_range.end = passage
            .text_range
            .end
            .checked_sub(self.old_byte)?
            .checked_add(self.new_byte)?;
        passage.header_line = self.line(passage.header_line)?;
        passage.target_line = match passage.target_line {
            Some(line) => Some(self.line(line)?),
            None => None,
        };
        Some(())
    }

    fn line(&self, line: u32) -> Option<u32> {
        line.checked_sub(self.old_line)?.checked_add(self.new_line)
    }

    pub(super) fn block(&self, block: &mut ScriptBlock) -> Option<()> {
        self.entries(&mut block.entries)
    }

    fn entries(&self, entries: &mut [ScriptEntry]) -> Option<()> {
        for entry in entries {
            match entry {
                ScriptEntry::Passage(passage) => self.passage(passage)?,
                ScriptEntry::Group { entries, .. } => self.entries(entries)?,
                ScriptEntry::Jump(_) | ScriptEntry::Effect(_) | ScriptEntry::Source(_) => {}
            }
        }
        Some(())
    }
}
