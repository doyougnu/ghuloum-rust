// final model: in this model we use the slicing of the previous model but type
// each arena for each constructor in our AST. The result is that we cannot
// confuse indices for different types and still get all the benefits of the
// arenas
use libc::{mmap, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};

use crate::infra::arena::{Arena, Idx, RangeIdx};
use crate::infra::types::GB;

pub enum Fixnum {
    Integer(i64),
    Float(f64), // TODO: add more
}
pub type FixnumIdx = Idx<Fixnum>;

pub struct String {
    // we store strings as raw bytes, so we need to define a slice
    pub start: u32,
    pub end: u32,
}

pub struct Symbol {
    pub raw: u32,
}

pub struct Cons {
    pub car: u32, // an index for the element
    pub cdr: u32, // an index for the next cons or null
}

pub struct Vector {
    pub start: u32,
    pub end: u32,
}

pub type StringIdx = RangeIdx<String>;
pub type SymbolIdx = Idx<Symbol>;
pub type ListIdx = Idx<Cons>;
pub type VectorIdx = RangeIdx<Vector>;

type VarBlob = u8;
type StrBlob = u8;
type ExprBlob = u32; // 32-bit so that a chunk aligns with an index pointer.
                     // Compound data types typically will store pointers so
                     // this matches 1:1

// Compiler Context manages all the mmap'd memory, it splits this memory up into
// typed arenas and inserts a PROT_NONE between the boundaries to seg fault on
// an overflow
pub struct CompilerContext {
    pub base: *mut u8, // the start of the entire OS block

    pub variables: Arena<VarBlob>,
    pub fixnums: Arena<Fixnum>,
    pub strings: Arena<StrBlob>, // raw bytes for strings
    pub lists: Arena<Cons>,
    pub vectors: Arena<ExprBlob>,
}

// START: TODO: insert PROT_NONE

impl CompilerContext {
    pub fn initialize(gb_to_reserve: usize) -> Self {
        let base: *mut u8 = unsafe {
            mmap(
                std::ptr::null_mut(),
                gb_to_reserve,
                PROT_READ | PROT_WRITE,      // we get read write permissions
                MAP_PRIVATE | MAP_ANONYMOUS, // PRIVATE + ANONYMOUS to reserve
                // the space then only get page
                // faults on a write (so we reserve
                // memory but don't write until
                // needed)
                -1,
                0,
            ) as *mut u8
        };
        assert!(!base.is_null());

        unsafe {
            let variables = Arena::<VarBlob>::new(base, 1 * GB);
            let fixnums = Arena::<Fixnum>::new(base.add(1 * GB) as *mut Fixnum, 1 * GB);
            let strings = Arena::<StrBlob>::new(base.add(2 * GB), 1 * GB);
            let lists = Arena::<Cons>::new(base.add(3 * GB) as *mut Cons, 1 * GB);
            let vectors = Arena::<ExprBlob>::new(base.add(4 * GB) as *mut ExprBlob, 2 * GB);
            Self {
                base,
                variables,
                fixnums,
                strings,
                lists,
                vectors,
            }
        }
    }
}

// TODO: implement the destructor for the compiler context
// impl<T> Drop for CompilerContext<T> {
//     fn drop(&mut self) {
//         unsafe {
//             let addr = self.base_ptr as *mut libc::c_void;
//             let size = self.capacity * std::mem::size_of::<T>();
//             munmap(addr, size);
//         }
//     }
// }

// a demonstration of how to represent data Variable = Variable String Ty
// pub struct Variable {
// name: StringID,
// ty: TypeId
// }

// just a demonstration of what built-in functions would look like
// pub struct Function {
// name: String,
// args: Vector,
// body: List,
// return_type: TypeId
// }
