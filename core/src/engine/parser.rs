//! Recursive-descent parser over the tokenizer's output.
//! Precedence (lowest → highest):
//!   `+ -`
//!   then `* / mod` (mod is the binary `%` form, or the word `mod`)
//!   then unary `-`
//!   then `^` (right-assoc)
//!   then postfix `!` and `%`
//!   then primary (numbers, constants, parens, function calls)
//!
//! The parser tolerates two classes of malformed input allowed by the
//! spec: a trailing binary operator that lacks a right operand, and
//! missing right parentheses at end of input.

use crate::engine::ast::Node;
use crate::engine::errors::CalcError;
use crate::engine::item::{BinOp, BinaryFunc, UnaryFunc};
use crate::engine::tokenizer::Token;

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

/// Parse a token stream into an AST. Returns Err(Undefined) on
/// unrecoverable structural problems such as an empty stream after
/// sanitisation, or on input the grammar cannot account for in full.
pub fn parse(toks: Vec<Token>) -> Result<Node, CalcError> {
    let mut p = Parser { toks, pos: 0 };
    // Drop any trailing binary operator (e.g., user ended with "+").
    while matches!(
        p.toks.last(),
        Some(Token::Op(_)) | Some(Token::Mod) | Some(Token::Comma)
    ) {
        p.toks.pop();
    }
    if p.toks.is_empty() {
        return Err(CalcError::Undefined);
    }
    let node = p.parse_expr()?;
    // Every token has to be accounted for. Without this check a stray
    // closer silently truncates the expression – `1+2)*100` parsed as
    // `1+2` and returned 3, with no indication that most of the input
    // had been thrown away.
    if p.pos != p.toks.len() {
        return Err(CalcError::Undefined);
    }
    Ok(node)
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }
    fn advance(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).copied();
        self.pos += 1;
        t
    }
    /// Consume a closing paren if present; missing RParen at end of
    /// input is tolerated per spec.
    fn eat_rparen(&mut self) {
        if matches!(self.peek(), Some(Token::RParen)) {
            self.pos += 1;
        }
    }

    /// Parse a right-hand operand, distinguishing "the operator simply
    /// has nothing after it" – which the spec tolerates – from "what
    /// follows is malformed", which is a real error.
    ///
    /// The distinction needs the position restored first: a failed
    /// parse has usually consumed tokens on its way down, so the
    /// decision has to be made against the input as it stood before the
    /// attempt. Without that, `5+()` looked identical to `5+` and
    /// silently evaluated to 5.
    fn parse_operand<F>(&mut self, mut parse: F) -> Result<Option<Node>, CalcError>
    where
        F: FnMut(&mut Self) -> Result<Node, CalcError>,
    {
        let start = self.pos;
        match parse(self) {
            Ok(node) => Ok(Some(node)),
            Err(e) => {
                self.pos = start;
                // Nothing left, or only a closer: the operand is
                // genuinely absent and the caller should stop here.
                if matches!(self.peek(), None | Some(Token::RParen)) {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// expr = term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<Node, CalcError> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Op(BinOp::Add) => BinOp::Add,
                Token::Op(BinOp::Sub) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let Some(right) = self.parse_operand(Self::parse_term)? else {
                break; // trailing operator: stop the loop
            };
            left = Node::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// term = unary (('*' | '/' | '%mod') unary)*
    fn parse_term(&mut self) -> Result<Node, CalcError> {
        let mut left = self.parse_unary()?;
        loop {
            let (is_mod, op) = match self.peek() {
                Some(Token::Op(BinOp::Mul)) => (false, BinOp::Mul),
                Some(Token::Op(BinOp::Div)) => (false, BinOp::Div),
                Some(Token::Mod) => (true, BinOp::Mul /* placeholder */),
                _ => break,
            };
            self.advance();
            let Some(right) = self.parse_operand(Self::parse_unary)? else {
                break;
            };
            left = if is_mod {
                Node::Mod(Box::new(left), Box::new(right))
            } else {
                Node::Bin(op, Box::new(left), Box::new(right))
            };
        }
        Ok(left)
    }

    /// unary = '-' unary | unaryFn (…) | binaryFn(…) | logN(…) | power
    fn parse_unary(&mut self) -> Result<Node, CalcError> {
        match self.peek() {
            Some(Token::Op(BinOp::Sub)) => {
                self.advance();
                let inner = self.parse_unary()?;
                Ok(Node::Neg(Box::new(inner)))
            }
            Some(Token::Op(BinOp::Add)) => {
                // Leading `+`: ignore sign.
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_power(),
        }
    }

    /// power = postfix ('^' unary)?    (right-assoc, right = unary so `2^-3` works)
    fn parse_power(&mut self) -> Result<Node, CalcError> {
        let base = self.parse_postfix()?;
        if matches!(self.peek(), Some(Token::Op(BinOp::Pow))) {
            self.advance();
            let Some(exp) = self.parse_operand(Self::parse_unary)? else {
                return Ok(base);
            };
            return Ok(Node::Bin(BinOp::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    /// postfix = primary ('!' | '%')*
    fn parse_postfix(&mut self) -> Result<Node, CalcError> {
        let mut e = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(Token::Factorial) => {
                    self.advance();
                    e = Node::Factorial(Box::new(e));
                }
                Some(Token::Percent) => {
                    self.advance();
                    e = Node::Percent(Box::new(e));
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// primary = number | const | '(' expr ')' | funcCall
    fn parse_primary(&mut self) -> Result<Node, CalcError> {
        let tok = self.advance().ok_or(CalcError::Undefined)?;
        match tok {
            Token::Num(n) => Ok(Node::Num(n)),
            Token::Const(c) => Ok(Node::Const(c)),
            Token::LParen => {
                let inner = self.parse_expr()?;
                self.eat_rparen();
                Ok(inner)
            }
            Token::UnaryFn(UnaryFunc::Log) => self.parse_log_call(),
            Token::UnaryFn(f) => self.parse_unary_call(f),
            Token::BinaryFn(BinaryFunc::Root) => self.parse_root_call(),
            Token::BinaryFn(BinaryFunc::LogBase) => {
                // Not emitted by tokenizer directly but kept for future use.
                self.parse_logbase_call()
            }
            Token::LogN(base) => self.parse_logn_call(base),
            // Operators or closing punctuation at primary position are treated
            // as missing operand; return a benign zero so higher rules can
            // detect and fall through, or translate to an error.
            _ => Err(CalcError::Undefined),
        }
    }

    fn parse_unary_call(&mut self, f: UnaryFunc) -> Result<Node, CalcError> {
        // Optional parens: sqrt(4) or sqrt 4
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let arg = self.parse_expr()?;
            self.eat_rparen();
            return Ok(Node::UnaryFn(f, Box::new(arg)));
        }
        // No paren: take the next postfix as the argument.
        let arg = self.parse_postfix()?;
        Ok(Node::UnaryFn(f, Box::new(arg)))
    }

    fn parse_log_call(&mut self) -> Result<Node, CalcError> {
        // `log` can be 1-arg (log10) or 2-arg (log base,value).
        if !matches!(self.peek(), Some(Token::LParen)) {
            // Bare `log x` form – 1 arg.
            let arg = self.parse_postfix()?;
            return Ok(Node::UnaryFn(UnaryFunc::Log, Box::new(arg)));
        }
        self.advance(); // consume '('
        let a = self.parse_expr()?;
        if matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            let b = self.parse_expr()?;
            self.eat_rparen();
            return Ok(Node::BinaryFn(
                BinaryFunc::LogBase,
                Box::new(a),
                Box::new(b),
            ));
        }
        self.eat_rparen();
        Ok(Node::UnaryFn(UnaryFunc::Log, Box::new(a)))
    }

    fn parse_logbase_call(&mut self) -> Result<Node, CalcError> {
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
        }
        let a = self.parse_expr()?;
        if matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
        }
        let b = self.parse_expr()?;
        self.eat_rparen();
        Ok(Node::BinaryFn(
            BinaryFunc::LogBase,
            Box::new(a),
            Box::new(b),
        ))
    }

    fn parse_root_call(&mut self) -> Result<Node, CalcError> {
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
        }
        let a = self.parse_expr()?;
        if matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
        }
        let b = self.parse_expr()?;
        self.eat_rparen();
        Ok(Node::BinaryFn(BinaryFunc::Root, Box::new(a), Box::new(b)))
    }

    fn parse_logn_call(&mut self, base: f64) -> Result<Node, CalcError> {
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let arg = self.parse_expr()?;
            self.eat_rparen();
            Ok(Node::LogN(base, Box::new(arg)))
        } else {
            let arg = self.parse_postfix()?;
            Ok(Node::LogN(base, Box::new(arg)))
        }
    }
}
