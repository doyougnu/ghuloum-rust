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

#[repr(u4)] // force the discriminant to be one nibble, TODO add more
            // (Word64,32,16) for example
pub enum Fixnum {
    Integer(i64),
    Float(f64),
}

#[repr(C)]
pub struct Pair {
    pub car: u32, // index into a value
    pub cdr: u32, // index into Arena<Pair>
}

// in this model, a list would be a u32 index into an Arena of Pairs. The car
// and the cdr would then be indices into other arenas or the pair arena again.
// To differentiate what values are being stored you then must define and
// scrutinize the u32 tags for the elements.
pub enum Expr1 {
    Bool(bool),
    Integer(i64), // and we keep fixnums as they are
    Float(f64),
    String(u32),
    Symbol(u32),
    List(u32), // index into Arena<Pair>
}
