// final model: in this model we use the slicing of the previous model but type
// each arena for each constructor in our AST. The result is that we cannot
// confuse indices for different types and still get all the benefits of the
// arenas

#[repr(0)]
pub enum Fixnum {
    Integer(i64),
    Float(f64), // TODO: add more
}

pub struct String {
    // we store strings as raw bytes, so we need to define a slice
    start: u32,
    end: u32,
}

pub struct Symbol {
    idx: u32,
}

pub struct Cons {
    car: u32, // an index for the element
    cdr: u32, // an index for the next cons or null
}

pub struct Vector {
    start: u32,
    end: u32,
}

pub struct Variable {
    // rep variables as strings
    idx: String,
}

type VarBlob = u8;
type StrBlob = u8;
type ExprBlob = u32; // 32-bit so that a chunk aligns with an index pointer.
                     // Compound data types typically will store pointers so
                     // this matches 1:1

// TODO: patch arena to type the indices, I believe then we can use type
// synonyms instead of newtypes
pub struct CompilerContext {
    variables: Arena<VarBlob>,
    fixnums: Arena<Fixnum>,
    string: Arena<StrBlob>, // raw bytes for strings
    lists: Arena<Cons>,
    vectors: Arena<ExprBlob>,
}

impl CompilerContext {
    pub fn initialize() -> Self {
        let variables = Arena::new(2);
        let fixnums = Arena::new(2);
        let strings = Arena::new(2);
        let lists = Arena::new(2);
        let vectors = Arena::new(2);
        Self {
            variables,
            fixnums,
            strings,
            lists,
            vectors,
        }
    }
}

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
