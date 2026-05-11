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
pub type VectorHdr = Slice<Expr>; // START: fix the vec types

pub struct Variable(pub Ptr<VariableHdr>);
pub struct String(pub Ptr<StringHdr>);
pub struct Symbol(pub Ptr<SymbolHdr>);
pub struct Vector(pub Ptr<VectorHdr>);
pub struct List(pub Ptr<Cons>);
pub struct Word(pub Ptr<AnyTy>);
pub struct Bool(pub Ptr<AnyTy>);

impl Variable {
    pub fn new(hdr: Ptr<VariableHdr>) -> Self {
        debug_assert!(hdr.idx <= INDEX_MASK);
        Self(Ptr::new((TAG_VARIABLE << TAG_SHIFT) | (hdr & INDEX_MASK)))
    }
}

impl String {
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

pub struct Cons {
    hd: Expr,
    tl: List,
}

impl Cons {
    pub fn new(hd: Expr, tl: List) -> Self {
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

impl Expr {
    pub fn tag(self) -> u32 {
        self.idx >> TAG_SHIFT
    }

    pub fn index(self) -> u32 {
        self.idx & INDEX_MASK
    }

    pub fn nil() -> Self {
        Ptr::new(TAG_FIXNUM | 0)
    }
    // For now we'll pack the fixnum into the index so we don't have to store it
    // TODO: detect and implement bignums
    pub fn fixnum(self) -> Self {
        debug_assert!(self.idx <= INDEX_MASK);
        Ptr::new((TAG_FIXNUM << TAG_SHIFT) | (self & INDEX_MASK))
    }

    pub fn cons(self) -> Self {
        debug_assert!(self.idx <= INDEX_MASK);
        Ptr::new((TAG_CONS << TAG_SHIFT) | (self & INDEX_MASK))
    }

    pub fn symbol(self) -> Self {
        debug_assert!(self.idx <= INDEX_MASK);
        Ptr::new((TAG_SYMBOL << TAG_SHIFT) | (self & INDEX_MASK))
    }

    // TODO: build in bool so there is only ever 2
    // and pack the bool in the pointer
    pub fn bool(self) -> Self {
        debug_assert!(self.idx <= INDEX_MASK);
        Ptr::new((TAG_BOOL << TAG_SHIFT) | (self & INDEX_MASK))
    }

    // A vector stores an index into a vector header arena, then the header
    // stores the pointers for the actual slice so we chase 2 pointers to get a
    // nice contiguous set of vals for the vector
    pub fn vector(self) -> Self {
        debug_assert!(self.idx <= INDEX_MASK);
        Ptr::new((TAG_SYMBOL << TAG_SHIFT) | (self & INDEX_MASK))
    }

    /****************************** projections ********************************/
    // each of these functions will check the tag then lift that information
    // into rusts type system
    pub fn as_fixnum(&self) -> i32 {
        debug_assert_eq!(self.tag(), TAG_FIXNUM);
        self.index() as i32
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
