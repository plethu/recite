use rowan::{Language, SyntaxNode};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ReciteLanguage {}

impl Language for ReciteLanguage {
    type Kind = ReciteSyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        ReciteSyntaxKind::from_raw(raw)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type ReciteSyntaxNode = SyntaxNode<ReciteLanguage>;

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ReciteSyntaxKind {
    Root = 0,
    Block = 1,
    Line = 2,
    Choice = 3,
    Effect = 4,
    Divert = 5,
    If = 6,
    Else = 7,
    Match = 8,
    Case = 9,
    Comment = 10,
    Prose = 11,
    Error = 12,

    BlockMarker = 100,
    LineMarker = 101,
    ChoiceMarker = 102,
    EffectMarker = 103,
    DivertMarker = 104,
    DirectiveMarker = 105,
    Ident = 106,
    Text = 107,
    Whitespace = 108,
    Newline = 109,
    CommentText = 110,
}

impl ReciteSyntaxKind {
    fn from_raw(raw: rowan::SyntaxKind) -> Self {
        match raw.0 {
            0 => Self::Root,
            1 => Self::Block,
            2 => Self::Line,
            3 => Self::Choice,
            4 => Self::Effect,
            5 => Self::Divert,
            6 => Self::If,
            7 => Self::Else,
            8 => Self::Match,
            9 => Self::Case,
            10 => Self::Comment,
            11 => Self::Prose,
            12 => Self::Error,
            100 => Self::BlockMarker,
            101 => Self::LineMarker,
            102 => Self::ChoiceMarker,
            103 => Self::EffectMarker,
            104 => Self::DivertMarker,
            105 => Self::DirectiveMarker,
            106 => Self::Ident,
            107 => Self::Text,
            108 => Self::Whitespace,
            109 => Self::Newline,
            110 => Self::CommentText,
            _ => Self::Error,
        }
    }
}

impl From<ReciteSyntaxKind> for rowan::SyntaxKind {
    fn from(kind: ReciteSyntaxKind) -> Self {
        rowan::SyntaxKind(kind as u16)
    }
}
