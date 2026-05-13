use recite_core::SourceSpan;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TokenKind {
    Ident(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    Comma,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Token {
    pub(super) kind: TokenKind,
    pub(super) span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TokenKindDiscriminant {
    Ident,
    String,
    Integer,
    Float,
    Boolean,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    Comma,
    End,
}

impl TokenKind {
    pub(super) fn discriminant(&self) -> TokenKindDiscriminant {
        match self {
            Self::Ident(_) => TokenKindDiscriminant::Ident,
            Self::String(_) => TokenKindDiscriminant::String,
            Self::Integer(_) => TokenKindDiscriminant::Integer,
            Self::Float(_) => TokenKindDiscriminant::Float,
            Self::Boolean(_) => TokenKindDiscriminant::Boolean,
            Self::And => TokenKindDiscriminant::And,
            Self::Or => TokenKindDiscriminant::Or,
            Self::Not => TokenKindDiscriminant::Not,
            Self::LeftParen => TokenKindDiscriminant::LeftParen,
            Self::RightParen => TokenKindDiscriminant::RightParen,
            Self::Comma => TokenKindDiscriminant::Comma,
            Self::End => TokenKindDiscriminant::End,
        }
    }
}
