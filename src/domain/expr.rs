use std::fmt;

use crate::infra::varena::Arena;

pub struct CompilerContext1 {
    // atoms
    // pub pairs: Arena<Pair>,
    pub symbols: Arena<u8>, // A "blob" arena for symbols
    pub strings: Arena<u8>, // A "blob" arena for raw bytes
    pub compounds: Arena<u8>,
    pub fixnums: Arena<Fixnum>,
}

impl CompilerContext1 {
    pub fn new() -> Self {
        Self {
            symbols: Arena::new(2), // 2GB
            strings: Arena::new(4),
            compounds: Arena::new(2),
            fixnums: Arena::new(2),
        }
    }
}

// TODO add more (Word64,32,16) for example
pub enum Fixnum {
    Integer(i64),
    Float(f64),
}

// What we are starting with for reference
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Atoms
    Bool(bool),
    Integer(i64),
    Float(f64),
    Char(char),
    Str(String),
    Symbol(String),
    Nil,

    // Compound
    List(Vec<Expr>),
    DottedList(Vec<Expr>, Box<Expr>), // (a b . c)
    Vector(Vec<Expr>),

    // Reader shorthands
    Quote(Box<Expr>),
    Quasiquote(Box<Expr>),
    Unquote(Box<Expr>),
    UnquoteSplicing(Box<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Bool(true) => write!(f, "#t"),
            Expr::Bool(false) => write!(f, "#f"),
            Expr::Integer(n) => write!(f, "{}", n),
            Expr::Float(n) => write!(f, "{}", n),
            Expr::Char(c) => write!(f, "#\\{}", c),
            Expr::Str(s) => write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Expr::Symbol(s) => write!(f, "{}", s),
            Expr::Nil => write!(f, "()"),
            Expr::List(xs) => {
                write!(f, "(")?;
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", x)?;
                }
                write!(f, ")")
            }
            Expr::DottedList(xs, tail) => {
                write!(f, "(")?;
                for x in xs {
                    write!(f, "{} ", x)?;
                }
                write!(f, ". {})", tail)
            }
            Expr::Vector(xs) => {
                write!(f, "#(")?;
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", x)?;
                }
                write!(f, ")")
            }
            Expr::Quote(e) => write!(f, "'{}", e),
            Expr::Quasiquote(e) => write!(f, "`{}", e),
            Expr::Unquote(e) => write!(f, ",{}", e),
            Expr::UnquoteSplicing(e) => write!(f, ",@{}", e),
        }
    }
}
