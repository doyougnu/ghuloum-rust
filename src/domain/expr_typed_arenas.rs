// final model: in this model we use the slicing of the previous model but type
// each arena for each constructor in our AST. The result is that we cannot
// confuse indices for different types and still get all the benefits of the
// arenas

use crate::infra::arena::{Ptr, Slice};

/****************************** Type level Tags ********************************/
pub struct VariableTy;
pub struct StringTy;
pub struct SymbolTy;
pub struct VectorTy;
pub struct ExprTy;
pub struct ListTy;
pub struct NumTy;
pub struct AnyTy;

/***************************** Runtime Values **********************************/
pub type VariableHdr = Slice<u8>;
pub type StringHdr = Slice<u8>;
pub type SymbolHdr = Slice<u8>;
pub type VectorHdr = Slice<Expr>;

#[derive(Clone, Copy)]
pub struct Variable(pub Ptr<VariableHdr>);

#[derive(Clone, Copy)]
pub struct SString(pub Ptr<StringHdr>);

#[derive(Clone, Copy)]
pub struct Symbol(pub Ptr<SymbolHdr>);

#[derive(Clone, Copy)]
pub struct Vector(pub Ptr<VectorHdr>);

#[derive(Clone, Copy)]
pub struct List(pub Ptr<Cons>);

#[derive(Clone, Copy)]
pub struct Word(pub Ptr<AnyTy>);

#[derive(Clone, Copy)]
pub struct Bool(pub Ptr<AnyTy>);

#[derive(Clone, Copy)]
pub struct Quote(pub Ptr<AnyTy>);

#[derive(Clone, Copy)]
pub struct QuasiQuote(pub Ptr<AnyTy>);

#[derive(Clone, Copy)]
pub struct Unquote(pub Ptr<AnyTy>);

#[derive(Clone, Copy)]
pub struct UnquoteSplicing(pub Ptr<AnyTy>);

// START: impl new and as_ functions for quote and friends

impl Variable {
    pub fn new(hdr: Ptr<VariableHdr>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_VARIABLE << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl SString {
    pub fn new(hdr: Ptr<StringHdr>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_STRING << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl Symbol {
    pub fn new(hdr: Ptr<SymbolHdr>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_SYMBOL << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl Vector {
    pub fn new(hdr: Ptr<VectorHdr>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_VECTOR << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl List {
    pub fn new(hdr: Ptr<Cons>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_CONS << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
    pub fn nil() -> Self {
        Self(Ptr::new(TAG_NIL << TAG_SHIFT))
    }

    pub fn is_nil(&self) -> bool {
        return (self.0.idx >> TAG_SHIFT) == TAG_NIL;
    }
}

impl Word {
    pub fn new(hdr: Ptr<AnyTy>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_FIXNUM << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl Bool {
    pub fn new(hdr: Ptr<AnyTy>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_BOOL << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

#[derive(Clone, Copy)]
pub struct Cons {
    pub hd: Expr,
    pub tl: Expr,
}

impl Cons {
    pub fn new(hd: Expr, tl: Expr) -> Self {
        Self { hd, tl }
    }
}

/***************************** Pointers ****************************************/
// 32 bits per value: [4-bit tag | 28-bit index into an arena]

// that yields 2^28 possible indices into an arena, so if our address space is
// defined as a 1GB arena then we have 1,073,741,824 total bits and unique
// addresses. With a val being 4 bits that means we can express 268,435,456
// indices. So a 29-bit address space gives us 513,870,912 indices which is at
// least twice as much as we really need. Hence we tag the top 4 bits to give an
// address space of 268,435,456 (~256MB). This forces a 1-1 mapping between
// addresses and memory in the address space. That is a special case of storing
// bools because the bool only needs 1-bit to be represented. Thus, if we were
// to store a byte and map the entire address space we would need:
// (total-num-of-addresses * sizeof(byte)) which would be 256MB * 8 which is
// ~2GB
pub type Expr = Ptr<AnyTy>;

const TAG_SHIFT: u32 = 28;
const INDEX_MASK: u32 = (1 << TAG_SHIFT) - 1;

const TAG_FIXNUM: u32 = 0b0000;
const TAG_CONS: u32 = 0b0001;
const TAG_SYMBOL: u32 = 0b0010;
const TAG_VARIABLE: u32 = 0b0011;
const TAG_BOOL: u32 = 0b0100;
const TAG_NIL: u32 = 0b0101;
const TAG_VECTOR: u32 = 0b0110;
const TAG_STRING: u32 = 0b0111;

// We define From to lose type information. This has to happen in the parser for
// example. But the benefit is that the parser functions like parse_string will
// be type safe while we can still write the recursive decent parser like we
// normally would since the pointers will still be tagged. Then we can match on
// the tag to reconstruct the type when needed.
impl From<SString> for Expr {
    fn from(s: SString) -> Self {
        debug_assert!((s.0.idx >> TAG_SHIFT) == TAG_STRING);
        Ptr::new(s.0.idx)
    }
}

impl From<Variable> for Expr {
    fn from(v: Variable) -> Self {
        debug_assert!((v.0.idx >> TAG_SHIFT) == TAG_VARIABLE);
        Ptr::new(v.0.idx)
    }
}

impl From<Symbol> for Expr {
    fn from(s: Symbol) -> Self {
        debug_assert!((s.0.idx >> TAG_SHIFT) == TAG_SYMBOL);
        Ptr::new(s.0.idx)
    }
}

impl From<Vector> for Expr {
    fn from(v: Vector) -> Self {
        debug_assert!((v.0.idx >> TAG_SHIFT) == TAG_VECTOR);
        Ptr::new(v.0.idx)
    }
}

impl From<List> for Expr {
    fn from(l: List) -> Self {
        debug_assert!((l.0.idx >> TAG_SHIFT) == TAG_CONS);
        Ptr::new(l.0.idx)
    }
}

impl From<Bool> for Expr {
    fn from(b: Bool) -> Self {
        debug_assert!((b.0.idx >> TAG_SHIFT) == TAG_BOOL);
        Ptr::new(b.0.idx)
    }
}

impl From<Word> for Expr {
    fn from(w: Word) -> Self {
        debug_assert!((w.0.idx >> TAG_SHIFT) == TAG_FIXNUM);
        Ptr::new(w.0.idx)
    }
}

impl Expr {
    #[inline(always)]
    pub fn tag(self) -> u32 {
        self.idx >> TAG_SHIFT
    }

    #[inline(always)]
    pub fn index(self) -> u32 {
        self.idx & INDEX_MASK
    }

    // TODO: remove?
    pub fn nil() -> Self {
        Ptr::new(TAG_FIXNUM | 0)
    }

    /****************************** projections ********************************/
    // each of these functions will check the tag then lift that information
    // into rusts type system
    pub fn as_fixnum(&self) -> i32 {
        debug_assert_eq!(self.tag(), TAG_FIXNUM);
        self.index() as i32
    }

    pub fn as_vector(self) -> Vector {
        debug_assert_eq!(self.tag(), TAG_VECTOR);
        Vector(self.cast::<VectorHdr>())
    }

    pub fn as_list(self) -> List {
        debug_assert_eq!(self.tag(), TAG_CONS);
        List(self.cast::<Cons>())
    }

    pub fn as_variable(self) -> Variable {
        debug_assert_eq!(self.tag(), TAG_VARIABLE);
        Variable(self.cast::<VariableHdr>())
    }

    pub fn as_symbol(self) -> Symbol {
        debug_assert_eq!(self.tag(), TAG_SYMBOL);
        Symbol(self.cast::<SymbolHdr>())
    }

    pub fn as_string(self) -> SString {
        debug_assert_eq!(self.tag(), TAG_STRING);
        SString(self.cast::<StringHdr>())
    }

    pub fn as_bool(self) -> Bool {
        debug_assert_eq!(self.tag(), TAG_BOOL);
        Bool(self)
    }
}

// a demonstration of how to represent data Variable = Variable String Ty
// pub struct Variable {
// name: StringID,
// ty: TypeId
// }

// just a demonstration of what built-in functions would look like
// pub struct Function {
// name: StringPtr,
// args: VectorPtr,
// body: ListPtr,
// return_type: TypeId
// }
