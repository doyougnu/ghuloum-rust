// scheme_parser.rs — A recursive descent parser for Scheme (R7RS subset)
// Handles: atoms, symbols, numbers, booleans, strings, characters,
//          lists, dotted pairs, vectors, and quote/quasiquote/unquote shorthands.

use crate::application::context::Context;
// use crate::domain::expr::Expr;
use crate::domain::expr_typed_arenas::*;

use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

type Result<T> = std::result::Result<T, ParseError>;

// ── Lexer ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LParen,
    RParen,
    LBracket, // [ — R7RS allows [] as list delimiters
    RBracket,
    HashLParen, // #( — vector literal
    Dot,
    Quote,
    Quasiquote,
    Unquote,
    UnquoteSplicing,
    Atom(String),
    StringLit(String),
    EOF,
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Lexer {
            chars: input.chars().peekable(),
            pos: 0,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // skip whitespace
            while self.peek().map_or(false, |c| c.is_whitespace()) {
                self.advance();
            }
            // skip line comments
            if self.peek() == Some(';') {
                while self.peek().map_or(false, |c| c != '\n') {
                    self.advance();
                }
            // skip block comments #| ... |#
            } else if self.peek() == Some('#') {
                // peek two chars — we need to clone the iterator momentarily
                // We'll just handle #| by consuming # and checking next
                // (Only do this if the NEXT char is |)
                let mut tmp = self.chars.clone();
                tmp.next(); // skip #
                if tmp.peek() == Some(&'|') {
                    self.advance(); // consume #
                    self.advance(); // consume |
                    loop {
                        match self.advance() {
                            None => break,
                            Some('|') if self.peek() == Some('#') => {
                                self.advance();
                                break;
                            }
                            _ => {}
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<Token> {
        let pos = self.pos;
        self.advance(); // consume opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(ParseError {
                        message: "unterminated string".into(),
                        position: pos,
                    })
                }
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('0') => s.push('\0'),
                    Some('a') => s.push('\x07'),
                    Some('b') => s.push('\x08'),
                    Some(c) => {
                        return Err(ParseError {
                            message: format!("unknown escape sequence: \\{}", c),
                            position: self.pos,
                        })
                    }
                    None => {
                        return Err(ParseError {
                            message: "unexpected EOF in string".into(),
                            position: self.pos,
                        })
                    }
                },
                Some(c) => s.push(c),
            }
        }
        Ok(Token::StringLit(s))
    }

    fn read_atom(&mut self) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || "()[]\";\n".contains(c) {
                break;
            }
            s.push(c);
            self.advance();
        }
        Token::Atom(s)
    }

    fn next_token(&mut self) -> Result<(Token, usize)> {
        self.skip_whitespace_and_comments();
        let pos = self.pos;
        match self.peek() {
            None => Ok((Token::EOF, pos)),
            Some('(') => {
                self.advance();
                Ok((Token::LParen, pos))
            }
            Some(')') => {
                self.advance();
                Ok((Token::RParen, pos))
            }
            Some('[') => {
                self.advance();
                Ok((Token::LBracket, pos))
            }
            Some(']') => {
                self.advance();
                Ok((Token::RBracket, pos))
            }
            Some('\'') => {
                self.advance();
                Ok((Token::Quote, pos))
            }
            Some('`') => {
                self.advance();
                Ok((Token::Quasiquote, pos))
            }
            Some(',') => {
                self.advance();
                if self.peek() == Some('@') {
                    self.advance();
                    Ok((Token::UnquoteSplicing, pos))
                } else {
                    Ok((Token::Unquote, pos))
                }
            }
            Some('"') => Ok((self.read_string()?, pos)),
            Some('#') => {
                self.advance(); // consume #
                match self.peek() {
                    Some('(') => {
                        self.advance();
                        Ok((Token::HashLParen, pos))
                    }
                    Some('t') => {
                        self.advance();
                        // allow #true
                        if self.peek().map_or(false, |c| c.is_alphabetic()) {
                            while self.peek().map_or(false, |c| c.is_alphabetic()) {
                                self.advance();
                            }
                        }
                        Ok((Token::Atom("#t".into()), pos))
                    }
                    Some('f') => {
                        self.advance();
                        if self.peek().map_or(false, |c| c.is_alphabetic()) {
                            while self.peek().map_or(false, |c| c.is_alphabetic()) {
                                self.advance();
                            }
                        }
                        Ok((Token::Atom("#f".into()), pos))
                    }
                    Some('\\') => {
                        self.advance(); // consume backslash
                        let mut name = String::new();
                        while let Some(c) = self.peek() {
                            if c.is_whitespace() || "()[]\";\n".contains(c) {
                                break;
                            }
                            name.push(c);
                            self.advance();
                        }
                        Ok((Token::Atom(format!("#\\{}", name)), pos))
                    }
                    _ => {
                        // fall through to atom (handles #b, #o, #d, #x numeric prefixes)
                        let mut s = String::from("#");
                        while let Some(c) = self.peek() {
                            if c.is_whitespace() || "()[]\";\n".contains(c) {
                                break;
                            }
                            s.push(c);
                            self.advance();
                        }
                        Ok((Token::Atom(s), pos))
                    }
                }
            }
            _ => Ok((self.read_atom(), pos)),
        }
    }
}

// ── Token stream ──────────────────────────────────────────────────────────────

struct TokenStream<'a> {
    lexer: Lexer<'a>,
    peeked: Option<(Token, usize)>,
}

impl<'a> TokenStream<'a> {
    fn new(input: &'a str) -> Self {
        TokenStream {
            lexer: Lexer::new(input),
            peeked: None,
        }
    }

    fn peek(&mut self) -> Result<&(Token, usize)> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    fn next(&mut self) -> Result<(Token, usize)> {
        if let Some(t) = self.peeked.take() {
            return Ok(t);
        }
        self.lexer.next_token()
    }

    // fn pos(&mut self) -> usize {
    // self.peeked.as_ref().map_or(self.lexer.pos, |(_, p)| *p)
    // }
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub struct Parser<'a> {
    tokens: TokenStream<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokens: TokenStream::new(input),
        }
    }

    /// Parse a single top-level expression.
    pub fn parse_expr(&mut self, ctx: &mut Context) -> Result<Expr> {
        let (tok, pos) = self.tokens.next()?;
        self.parse_from(ctx, tok, pos)
    }

    fn parse_from(&mut self, ctx: &mut Context, tok: Token, pos: usize) -> Result<Expr> {
        match tok {
            Token::EOF => Err(ParseError {
                message: "unexpected end of input".into(),
                position: pos,
            }),

            Token::Atom(s) => Ok(parse_atom(&s, pos)?),

            Token::StringLit(s) => Ok(ctx.alloc_string(s).into()),

            Token::Quote => {
                let e = self.parse_expr(ctx)?;
                Ok(Expr::Quote(Box::new(e)))
            }
            Token::Quasiquote => {
                let e = self.parse_expr(ctx)?;
                Ok(Expr::Quasiquote(Box::new(e)))
            }
            Token::Unquote => {
                let e = self.parse_expr(ctx)?;
                Ok(Expr::Unquote(Box::new(e)))
            }
            Token::UnquoteSplicing => {
                let e = self.parse_expr(ctx)?;
                Ok(Expr::UnquoteSplicing(Box::new(e)))
            }

            Token::LParen | Token::LBracket => {
                let close = if tok == Token::LParen {
                    Token::RParen
                } else {
                    Token::RBracket
                };
                let lst = self.parse_list(ctx, close, pos);
            }

            Token::HashLParen => self.parse_vector(ctx, pos),

            Token::RParen | Token::RBracket => Err(ParseError {
                message: "unexpected closing delimiter".into(),
                position: pos,
            }),

            Token::Dot => Err(ParseError {
                message: "unexpected dot".into(),
                position: pos,
            }),
        }
    }

    /// Parse a list (or dotted pair) body after the opening paren.
    fn parse_list(&mut self, ctx: &mut Context, close: Token, open_pos: usize) -> Result<List> {
        let mut items: Vec<Expr> = Vec::new();

        loop {
            // Peek at next token
            let (tok, pos) = {
                let (t, p) = self.tokens.peek()?.clone();
                (t, p)
            };

            if tok == close {
                self.tokens.next()?; // consume closing delimiter
                if items.is_empty() {
                    // then we have an empty list
                    return Ok(List::nil());
                }
                return Ok(ctx.alloc_list(items.as_slice()));
            }

            if tok == Token::EOF {
                return Err(ParseError {
                    message: "unterminated list".into(),
                    position: open_pos,
                });
            }

            if tok == Token::Dot {
                self.tokens.next()?; // consume dot
                if items.is_empty() {
                    return Err(ParseError {
                        message: "dot at start of list".into(),
                        position: pos,
                    });
                }
                let tail = self.parse_expr(ctx)?;
                // expect closing delimiter
                let (next_tok, next_pos) = self.tokens.next()?;
                if next_tok != close {
                    return Err(ParseError {
                        message: format!(
                            "expected closing delimiter after dotted tail, got {:?}",
                            next_tok
                        ),
                        position: next_pos,
                    });
                }
                return Ok(Expr::DottedList(items, Box::new(tail)));
            }

            // regular element
            let e = self.parse_expr(ctx)?;
            items.push(e);
        }
    }

    /// Parse a vector literal body after #(.
    fn parse_vector(&mut self, ctx: &mut Context, open_pos: usize) -> Result<Expr> {
        let mut items: Vec<Expr> = Vec::new();
        loop {
            let (tok, _pos) = self.tokens.peek()?.clone();
            match tok {
                Token::RParen => {
                    self.tokens.next()?;
                    return Ok(Expr::nil());
                }
                Token::EOF => {
                    return Err(ParseError {
                        message: "unterminated vector".into(),
                        position: open_pos,
                    });
                }
                _ => items.push(self.parse_expr(ctx)?),
            }
        }
    }

    /// Parse all expressions until EOF, a scheme program is a list
    pub fn parse_all(&mut self, ctx: &mut Context) -> Result<Vector> {
        loop {
            let (tok, _pos) = self.tokens.peek()?.clone();
            if tok == Token::EOF {
                break;
            }

            self.parse_expr(ctx)?;
        }
        Ok(exprs)
    }
}

// ── Atom parsing ──────────────────────────────────────────────────────────────

fn parse_atom(s: &str, pos: usize) -> Result<Expr> {
    // Booleans
    if s == "#t" || s == "#true" {
        return Ok(Expr::Bool(true));
    }
    if s == "#f" || s == "#false" {
        return Ok(Expr::Bool(false));
    }

    // Characters  #\a  #\space  #\newline  #\tab  #\nul  #\null
    if let Some(rest) = s.strip_prefix("#\\") {
        let ch = match rest {
            "space" => ' ',
            "newline" => '\n',
            "tab" => '\t',
            "return" => '\r',
            "nul" | "null" => '\0',
            "alarm" => '\x07',
            "backspace" => '\x08',
            "escape" => '\x1b',
            "delete" => '\x7f',
            s if s.len() == 1 => s.chars().next().unwrap(),
            other => {
                return Err(ParseError {
                    message: format!("unknown character name: #\\{}", other),
                    position: pos,
                })
            }
        };
        return Ok(Expr::Char(ch));
    }

    // Numeric prefixes  #b (binary)  #o (octal)  #d (decimal)  #x (hex)
    if let Some(rest) = s.strip_prefix("#b") {
        return i64::from_str_radix(rest, 2)
            .map(Expr::Integer)
            .map_err(|_| ParseError {
                message: format!("invalid binary number: {}", s),
                position: pos,
            });
    }
    if let Some(rest) = s.strip_prefix("#o") {
        return i64::from_str_radix(rest, 8)
            .map(Expr::Integer)
            .map_err(|_| ParseError {
                message: format!("invalid octal number: {}", s),
                position: pos,
            });
    }
    if let Some(rest) = s.strip_prefix("#d") {
        return rest
            .parse::<i64>()
            .map(Expr::Integer)
            .map_err(|_| ParseError {
                message: format!("invalid decimal number: {}", s),
                position: pos,
            });
    }
    if let Some(rest) = s.strip_prefix("#x") {
        return i64::from_str_radix(rest, 16)
            .map(Expr::Integer)
            .map_err(|_| ParseError {
                message: format!("invalid hex number: {}", s),
                position: pos,
            });
    }

    // Integer
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Expr::Integer(n));
    }

    // Float (also handle +inf.0, -inf.0, +nan.0 per R7RS)
    match s {
        "+inf.0" => return Ok(Expr::Float(f64::INFINITY)),
        "-inf.0" => return Ok(Expr::Float(f64::NEG_INFINITY)),
        "+nan.0" | "-nan.0" => return Ok(Expr::Float(f64::NAN)),
        _ => {}
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Expr::Float(f));
    }

    // Everything else is a symbol (case-folded per R5RS; R7RS is case-sensitive)
    Ok(Expr::Symbol(s.to_string()))
}

// ── Convenience top-level functions ───────────────────────────────────────────

/// Parse a single expression from a string.
pub fn parse(ctx: &mut Context, input: &str) -> Result<Expr> {
    Parser::new(input).parse_expr(ctx)
}

/// Parse all expressions from a string (e.g. a whole source file).
pub fn parse_all(input: &str) -> Result<Expr> {
    Parser::new(input).parse_all()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atoms() {
        assert_eq!(parse("42").unwrap(), Expr::Integer(42));
        assert_eq!(parse("-7").unwrap(), Expr::Integer(-7));
        assert_eq!(parse("3.14").unwrap(), Expr::Float(3.14));
        assert_eq!(parse("#t").unwrap(), Expr::Bool(true));
        assert_eq!(parse("#false").unwrap(), Expr::Bool(false));
        assert_eq!(parse("hello").unwrap(), Expr::Symbol("hello".into()));
        assert_eq!(parse("+").unwrap(), Expr::Symbol("+".into()));
    }

    #[test]
    fn test_strings() {
        assert_eq!(
            parse(r#""hello world""#).unwrap(),
            Expr::Str("hello world".into())
        );
        assert_eq!(
            parse(r#""tab\there""#).unwrap(),
            Expr::Str("tab\there".into())
        );
    }

    #[test]
    fn test_chars() {
        assert_eq!(parse(r"#\a").unwrap(), Expr::Char('a'));
        assert_eq!(parse(r"#\space").unwrap(), Expr::Char(' '));
        assert_eq!(parse(r"#\newline").unwrap(), Expr::Char('\n'));
    }

    #[test]
    fn test_nil() {
        assert_eq!(parse("()").unwrap(), Expr::Nil);
    }

    #[test]
    fn test_list() {
        assert_eq!(
            parse("(1 2 3)").unwrap(),
            Expr::List(vec![Expr::Integer(1), Expr::Integer(2), Expr::Integer(3)])
        );
    }

    #[test]
    fn test_nested() {
        let r = parse("(define (square x) (* x x))").unwrap();
        assert!(matches!(r, Expr::List(_)));
    }

    #[test]
    fn test_dotted_pair() {
        let r = parse("(1 . 2)").unwrap();
        assert_eq!(
            r,
            Expr::DottedList(vec![Expr::Integer(1)], Box::new(Expr::Integer(2)))
        );
    }

    #[test]
    fn test_quote_shorthand() {
        let r = parse("'foo").unwrap();
        assert_eq!(r, Expr::Quote(Box::new(Expr::Symbol("foo".into()))));
    }

    #[test]
    fn test_quasiquote() {
        let r = parse("`(a ,b ,@c)").unwrap();
        assert!(matches!(r, Expr::Quasiquote(_)));
    }

    #[test]
    fn test_vector() {
        let r = parse("#(1 2 3)").unwrap();
        assert_eq!(
            r,
            Expr::Vector(vec![Expr::Integer(1), Expr::Integer(2), Expr::Integer(3)])
        );
    }

    #[test]
    fn test_radix() {
        assert_eq!(parse("#b1010").unwrap(), Expr::Integer(10));
        assert_eq!(parse("#o17").unwrap(), Expr::Integer(15));
        assert_eq!(parse("#xff").unwrap(), Expr::Integer(255));
    }

    #[test]
    fn test_comments() {
        assert_eq!(parse("; this is a comment\n42").unwrap(), Expr::Integer(42));
    }

    #[test]
    fn test_parse_all() {
        let exprs = parse_all("1 2 (+ 3 4)").unwrap();
        assert_eq!(exprs.len(), 3);
    }

    #[test]
    fn test_error_unterminated_list() {
        assert!(parse("(1 2").is_err());
    }

    #[test]
    fn test_error_unexpected_close() {
        assert!(parse(")").is_err());
    }
}
