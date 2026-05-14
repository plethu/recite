use crate::{BlockId, ChoiceId, LineId};

use super::{BlockIndex, ChoiceIndex, CompiledValueError, LineIndex};

macro_rules! define_lookup_table {
    ($table:ident, $entry:ident, $id:ty, $index:ty, $name:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $entry {
            pub id: $id,
            pub index: $index,
        }

        #[derive(Clone, Debug, Default, Eq, PartialEq)]
        pub struct $table {
            entries: Vec<$entry>,
        }

        impl $table {
            pub fn new(entries: Vec<$entry>) -> Result<Self, CompiledValueError> {
                validate_lookup_order(entries.iter().map(|entry| entry.id.as_str()), $name)?;

                Ok(Self { entries })
            }

            pub fn iter(&self) -> impl Iterator<Item = &$entry> {
                self.entries.iter()
            }

            #[must_use]
            pub fn as_slice(&self) -> &[$entry] {
                &self.entries
            }

            #[must_use]
            pub fn len(&self) -> usize {
                self.entries.len()
            }

            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

define_lookup_table!(
    BlockLookupTable,
    BlockLookupEntry,
    BlockId,
    BlockIndex,
    "block"
);
define_lookup_table!(LineLookupTable, LineLookupEntry, LineId, LineIndex, "line");
define_lookup_table!(
    ChoiceLookupTable,
    ChoiceLookupEntry,
    ChoiceId,
    ChoiceIndex,
    "choice"
);

fn validate_lookup_order<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    table: &'static str,
) -> Result<(), CompiledValueError> {
    let mut previous: Option<&'a str> = None;

    for current in ids {
        if let Some(previous) = previous
            && previous >= current
        {
            return Err(CompiledValueError::UnsortedLookupTable {
                table,
                previous: previous.to_owned(),
                current: current.to_owned(),
            });
        }

        previous = Some(current);
    }

    Ok(())
}
