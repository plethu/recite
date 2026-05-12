use crate::syntax::ReciteSyntaxKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatementMarker {
    Block,
    Line,
    Choice,
    Effect,
    Divert,
    If,
    Else,
    Match,
    Case,
    Comment,
}

impl StatementMarker {
    pub(crate) fn parse(trimmed: &str) -> Option<Self> {
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with(Self::Block.text()) {
            return Some(Self::Block);
        }

        if trimmed.starts_with(Self::Divert.text()) {
            return Some(Self::Divert);
        }

        match trimmed.as_bytes()[0] {
            b'>' => Some(Self::Line),
            b'?' => Some(Self::Choice),
            b'!' => Some(Self::Effect),
            b'#' => Some(Self::Comment),
            b':' if trimmed.starts_with(Self::If.text()) => Some(Self::If),
            b':' if trimmed.starts_with(Self::Else.text()) => Some(Self::Else),
            b':' if trimmed.starts_with(Self::Match.text()) => Some(Self::Match),
            b':' if trimmed.starts_with(Self::Case.text()) => Some(Self::Case),
            _ => None,
        }
    }

    pub(crate) const fn text(self) -> &'static str {
        match self {
            Self::Block => "::",
            Self::Line => ">",
            Self::Choice => "?",
            Self::Effect => "!",
            Self::Divert => "->",
            Self::If => ":if",
            Self::Else => ":else",
            Self::Match => ":match",
            Self::Case => ":case",
            Self::Comment => "#",
        }
    }

    pub(crate) const fn syntax_kind(self) -> ReciteSyntaxKind {
        match self {
            Self::Block => ReciteSyntaxKind::Block,
            Self::Line => ReciteSyntaxKind::Line,
            Self::Choice => ReciteSyntaxKind::Choice,
            Self::Effect => ReciteSyntaxKind::Effect,
            Self::Divert => ReciteSyntaxKind::Divert,
            Self::If => ReciteSyntaxKind::If,
            Self::Else => ReciteSyntaxKind::Else,
            Self::Match => ReciteSyntaxKind::Match,
            Self::Case => ReciteSyntaxKind::Case,
            Self::Comment => ReciteSyntaxKind::Comment,
        }
    }

    pub(crate) const fn marker_syntax_kind(self) -> ReciteSyntaxKind {
        match self {
            Self::Block => ReciteSyntaxKind::BlockMarker,
            Self::Line => ReciteSyntaxKind::LineMarker,
            Self::Choice => ReciteSyntaxKind::ChoiceMarker,
            Self::Effect => ReciteSyntaxKind::EffectMarker,
            Self::Divert => ReciteSyntaxKind::DivertMarker,
            Self::If | Self::Else | Self::Match | Self::Case => ReciteSyntaxKind::DirectiveMarker,
            Self::Comment => ReciteSyntaxKind::CommentText,
        }
    }

    pub(crate) const fn is_unsupported_lowering(self) -> bool {
        matches!(
            self,
            Self::Choice
                | Self::Effect
                | Self::Divert
                | Self::If
                | Self::Else
                | Self::Match
                | Self::Case
        )
    }
}
