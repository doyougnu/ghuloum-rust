// next model: Expr2 is unified to a u32 and we pointer tag as much as possible
// to discriminate values. We reserve 3 bits so that we can store 500million
// indices and then we have 3 arenas: 1 for strings, 1 for symbols, 1 for blobs
// of data the blobs are 4-byte data chunks where the first byte is the tag (3
// bits) and the length (5 bits). For fixnums we store the fixnum directly in
// the index which is scrutinized by its tag
// 000 => fixnums
// 001 => Strings
// 010 => Symbols
// 011 => Lists (and a cons pair) with a Pair arena
// 100 => Vecs blob arenas
// 101 => quote or unquote
// 110 => unquote
// 111 => quasiquote

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

pub const TAG_MASK: u32 = 0b111; // 3-bits for the tag
pub enum Tag {
    Fixnum = 0,
    List = 1,
    Vector = 2,
    String = 3,
    Symbol = 4,
    Nil = 7,
}

/// An expr is a tagged index into some arena
pub struct Expr2(u32);
impl Expr2(u32) {
    pub fn mk(index: u32, tag: Tag) -> Self {
        Value((index << 3) | (tag as u32))
    }

    pub fn mk_fixnum(n: i32) -> Self {
        Value((n as u32) << 3 | (Tag::Fixnum as u32))
    }

    pub fn tag(self) -> Tag {
        match (self.0 & TAG_MASK) {
            0 => Tag::Fixnum,
            1 => Tag::List,
            2 => Tag::Vector,
            3 => Tag::String,
            4 => Tag::Symbol,
            7 => Tag::Nil,
            _ => unreachable!(),
        }
    }

    pub fn index(self) -> u32 {
        self.0 >> 3
    }
}
