macro_rules! define_table_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            #[must_use]
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_u32(self) -> u32 {
                self.0
            }
        }
    };
}

define_table_index!(SourceFileIndex);
define_table_index!(BlockIndex);
define_table_index!(StatementIndex);
define_table_index!(LineIndex);
define_table_index!(ChoiceIndex);
define_table_index!(SpeakerIndex);
define_table_index!(MetadataIndex);
define_table_index!(EffectIndex);
define_table_index!(SourceMapIndex);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TableRange<I> {
    pub start: I,
    pub len: u32,
}

impl<I> TableRange<I> {
    #[must_use]
    pub const fn new(start: I, len: u32) -> Self {
        Self { start, len }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

pub type StatementRange = TableRange<StatementIndex>;
pub type ChoiceRange = TableRange<ChoiceIndex>;
pub type MetadataRange = TableRange<MetadataIndex>;
