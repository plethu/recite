use super::ast::{Binary, Expr, Unary};

const MAX_EXPRESSION_DEPTH: usize = 128;
const MAX_EXPRESSION_NODES: usize = 512;

pub(super) fn parse_expression(input: &str) -> Result<Expr, ()> {
    let mut parser = Parser {
        input: input.as_bytes(),
        position: 0,
        nodes: 0,
    };
    let expression = parser.conditional(0)?;
    parser.skip_whitespace();
    (parser.position == parser.input.len())
        .then_some(expression)
        .ok_or(())
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
    nodes: usize,
}

impl Parser<'_> {
    fn conditional(&mut self, depth: usize) -> Result<Expr, ()> {
        let condition = self.or(depth)?;
        if !self.consume("?") {
            return Ok(condition);
        }
        if depth >= MAX_EXPRESSION_DEPTH {
            return Err(());
        }
        let when_true = self.conditional(depth + 1)?;
        if !self.consume(":") {
            return Err(());
        }
        let when_false = self.conditional(depth + 1)?;
        self.node(Expr::Conditional(
            Box::new(condition),
            Box::new(when_true),
            Box::new(when_false),
        ))
    }

    fn or(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::and, |parser| {
            parser.consume("||").then_some(Binary::Or)
        })
    }

    fn and(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::equality, |parser| {
            parser.consume("&&").then_some(Binary::And)
        })
    }

    fn equality(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::relation, |parser| {
            if parser.consume("==") {
                Some(Binary::Eq)
            } else if parser.consume("!=") {
                Some(Binary::Ne)
            } else {
                None
            }
        })
    }

    fn relation(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::addition, |parser| {
            if parser.consume("<=") {
                Some(Binary::Le)
            } else if parser.consume(">=") {
                Some(Binary::Ge)
            } else if parser.consume("<") {
                Some(Binary::Lt)
            } else if parser.consume(">") {
                Some(Binary::Gt)
            } else {
                None
            }
        })
    }

    fn addition(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::multiplication, |parser| {
            if parser.consume("+") {
                Some(Binary::Add)
            } else if parser.consume("-") {
                Some(Binary::Sub)
            } else {
                None
            }
        })
    }

    fn multiplication(&mut self, depth: usize) -> Result<Expr, ()> {
        self.binary(depth, Self::unary, |parser| {
            if parser.consume("*") {
                Some(Binary::Mul)
            } else if parser.consume("/") {
                Some(Binary::Div)
            } else if parser.consume("%") {
                Some(Binary::Rem)
            } else {
                None
            }
        })
    }

    fn binary(
        &mut self,
        depth: usize,
        parse_operand: fn(&mut Self, usize) -> Result<Expr, ()>,
        mut consume_operator: impl FnMut(&mut Self) -> Option<Binary>,
    ) -> Result<Expr, ()> {
        let mut expression = parse_operand(self, depth)?;
        while let Some(operator) = consume_operator(self) {
            let right = parse_operand(self, depth)?;
            expression = self.node(Expr::Binary(
                operator,
                Box::new(expression),
                Box::new(right),
            ))?;
        }
        Ok(expression)
    }

    fn unary(&mut self, depth: usize) -> Result<Expr, ()> {
        let operator = if self.consume("!") {
            Some(Unary::Not)
        } else if self.consume("+") {
            Some(Unary::Plus)
        } else if self.consume("-") {
            Some(Unary::Minus)
        } else {
            None
        };
        if let Some(operator) = operator {
            if depth >= MAX_EXPRESSION_DEPTH {
                return Err(());
            }
            let expression = self.unary(depth + 1)?;
            return self.node(Expr::Unary(operator, Box::new(expression)));
        }
        if self.consume("(") {
            if depth >= MAX_EXPRESSION_DEPTH {
                return Err(());
            }
            let expression = self.conditional(depth + 1)?;
            if !self.consume(")") {
                return Err(());
            }
            return Ok(expression);
        }
        if self.consume_identifier("n") {
            return self.node(Expr::N);
        }
        self.skip_whitespace();
        let start = self.position;
        while self
            .input
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        if start == self.position {
            return Err(());
        }
        let value = std::str::from_utf8(&self.input[start..self.position])
            .map_err(|_| ())?
            .parse()
            .map_err(|_| ())?;
        self.node(Expr::Number(value))
    }

    fn consume(&mut self, token: &str) -> bool {
        self.skip_whitespace();
        let token = token.as_bytes();
        if self
            .input
            .get(self.position..)
            .is_some_and(|rest| rest.starts_with(token))
        {
            self.position += token.len();
            true
        } else {
            false
        }
    }

    fn consume_identifier(&mut self, token: &str) -> bool {
        self.skip_whitespace();
        let start = self.position;
        if !self.consume(token) {
            return false;
        }
        if self
            .input
            .get(self.position)
            .is_some_and(|value| value.is_ascii_alphanumeric() || *value == b'_')
        {
            self.position = start;
            return false;
        }
        true
    }

    fn skip_whitespace(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }

    fn node(&mut self, expression: Expr) -> Result<Expr, ()> {
        self.nodes = self.nodes.checked_add(1).ok_or(())?;
        if self.nodes > MAX_EXPRESSION_NODES {
            return Err(());
        }
        Ok(expression)
    }
}
