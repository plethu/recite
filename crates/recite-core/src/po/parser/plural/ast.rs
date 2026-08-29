#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Expr {
    Number(i64),
    N,
    Unary(Unary, Box<Self>),
    Binary(Binary, Box<Self>, Box<Self>),
    Conditional(Box<Self>, Box<Self>, Box<Self>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Unary {
    Not,
    Plus,
    Minus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Binary {
    Or,
    And,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}
