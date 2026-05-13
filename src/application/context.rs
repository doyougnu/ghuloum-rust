use crate::domain::expr_typed_arenas::*;
use crate::infra::arena::*;
use crate::infra::types::*;
use libc::{mmap, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};

// Compiler Context manages all the mmap'd memory, it splits this memory up into
// typed arenas and inserts a 4k PROT_NONE page between the boundaries to seg
// fault on an overflow
pub struct Context {
    pub base: *mut u8, // the start of the entire OS block

    pub variable_hdr: Arena<VariableHdr>,
    pub symbol_hdr: Arena<SymbolHdr>,
    pub string_hdr: Arena<StringHdr>,
    pub vector_hdr: Arena<VectorHdr>,

    pub variables: Arena<u8>,
    pub symbols: Arena<u8>,
    pub strings: Arena<u8>,
    pub lists: Arena<Cons>,

    pub exprs: Arena<Expr>,
}

impl Context {
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

        let guard = |base: *mut u8| -> () {
            unsafe {
                libc::mprotect(base as *mut libc::c_void, GUARD_PAGE, libc::PROT_NONE);
            }
        };
        const GUARD_PAGE: usize = 4096;

        unsafe {
            /************************** headers ***************************/
            // headers are the 1st GiB, 256 MiB per header
            // var header is base + offset of 256MiB and so on
            // we hardcode the offsets for now
            let var_hdr_end = 256 * MiB;
            let variable_hdr = Arena::<VariableHdr>::new(base as *mut VariableHdr, 256 * MiB);
            guard(base.add(var_hdr_end - GUARD_PAGE));

            let sym_hdr_end = 512 * MiB;
            let symbol_hdr =
                Arena::<SymbolHdr>::new(base.add(var_hdr_end) as *mut SymbolHdr, 256 * MiB);
            guard(base.add(sym_hdr_end - GUARD_PAGE));

            let str_hdr_end = 768 * MiB;
            let string_hdr =
                Arena::<StringHdr>::new(base.add(sym_hdr_end) as *mut StringHdr, 256 * MiB);
            guard(base.add(str_hdr_end - GUARD_PAGE));

            let vec_hdr_end = 512 * MiB;
            let vector_hdr =
                Arena::<VectorHdr>::new(base.add(str_hdr_end) as *mut VectorHdr, 256 * MiB);
            guard(base.add(vec_hdr_end - GUARD_PAGE));

            /************************** payloads ***************************/
            // payloads start at 1GiB, and  we alloc 1 GiB for each payload
            // variables are from 1GiB ---> 2GiB
            let vars_end = 2 * GiB;
            let variables = Arena::<u8>::new(base.add(1 * GiB), 1 * GiB);
            guard(base.add(vars_end - GUARD_PAGE));

            // symbols from 2GiB --> 3GiB
            let syms_end = 3 * GiB;
            let symbols = Arena::<u8>::new(base.add(vars_end), 1 * GiB);
            guard(base.add(syms_end - GUARD_PAGE));

            // strings from 3GiB --> 4GiB
            let strs_end = 4 * GiB;
            let strings = Arena::<u8>::new(base.add(syms_end), 1 * GiB);
            guard(base.add(strs_end - GUARD_PAGE));

            // lists from 4GiB --> 5GiB
            let lists_end = 5 * GiB;
            let lists = Arena::<Cons>::new(base.add(lists_end) as *mut Cons, 1 * GiB);
            guard(base.add(lists_end - GUARD_PAGE));

            // vecs from 5GiB --> 6GiB
            let exprs_end = 6 * GiB;
            let exprs = Arena::<Expr>::new(base.add(exprs_end) as *mut Expr, 1 * GiB);
            guard(base.add(exprs_end - GUARD_PAGE));

            Self {
                base,
                // headers
                variable_hdr,
                symbol_hdr,
                string_hdr,
                vector_hdr,
                // payloads
                variables,
                symbols,
                strings,
                lists,
                exprs,
            }
        }
    }

    /*  Note [How we tag]
     ** We get a Ptr<T> back from the arena, but this ptr is untagged. We have
     ** carefully managed the address space to guarantee that the top 4-bits are
     ** empty.
     */
    pub fn alloc_var(&mut self, var: std::string::String) -> Variable {
        // first we alloc the payload to get the header info
        let var_hdr = self.variables.alloc_range(var.as_bytes());
        // now we alloc and tag the header
        let hdr_ptr = self.variable_hdr.alloc(var_hdr);
        Variable::new(hdr_ptr)
    }

    pub fn alloc_vector(&mut self, vec: Vec<Expr>) -> Vector {
        let vec_hdr = self.exprs.alloc_range(vec.as_slice());
        Vector::new(self.vector_hdr.alloc(vec_hdr))
    }

    pub fn alloc_symbol(&mut self, sym: std::string::String) -> Symbol {
        let sym_hdr = self.symbols.alloc_range(sym.as_bytes());
        Symbol::new(self.symbol_hdr.alloc(sym_hdr))
    }

    pub fn alloc_string(&mut self, str: std::string::String) -> SString {
        let str_hdr = self.strings.alloc_range(str.as_bytes());
        SString::new(self.string_hdr.alloc(str_hdr))
    }

    pub fn alloc_list(&mut self, list: &[Expr]) -> List {
        // these have all been alloc'd. If I have a pointer, which is what Expr
        // is, then the vals exist in some arena already. So this is really
        // foldr over the slice to yield: `val (val (val ...)))`
        match list {
            [head, tail @ ..] => {
                let rest = self.alloc_list(tail);
                // TODO: Ick but maybe we can type the list correctly at some point?
                List::new(self.lists.alloc(Cons::new(*head, rest.0.cast::<AnyTy>())))
            }
            // for the nil case: Nil is tagged in the pointer, so we are
            // constructing a pointer that doesn't actually point to anything by
            // casting to a Cons. This satisfies the type system while keeping
            // the invariants of the runtime intact
            [] => List::new(Expr::nil().cast::<Cons>()),
        }
    }

    pub fn get_vector(&self, exp: Vector) -> &[Expr] {
        let hdr = self.vector_hdr.get(exp.0);
        self.exprs.get_range(*hdr)
    }

    pub fn get_list(&self, lst: List) -> Option<&Cons> {
        // let mut ptr: List = lst;
        // let mut results: Vec<Expr>; // unfortunately need to use a vec
        // while !ptr.is_nil() {
        // let cons = self.lists.get(ptr.0);
        // results.push(cons.hd.clone());
        // ptr = cons.tl.cast::<Cons>();
        // }
        if lst.is_nil() {
            return None;
        } else {
            return Some(self.lists.get(lst.0));
        }
    }

    pub fn get_symbol(&self, sym: Symbol) -> &[u8] {
        let hdr = self.symbol_hdr.get(sym.0);
        self.symbols.get_range(*hdr)
    }

    pub fn get_string(&self, str: SString) -> &[u8] {
        let hdr = self.string_hdr.get(str.0);
        self.strings.get_range(*hdr)
    }

    pub fn get_variable(&self, v: Variable) -> &[u8] {
        let hdr = self.variable_hdr.get(v.0);
        self.variables.get_range(*hdr)
    }

    pub fn get_bool(&self, b: Bool) -> bool {
        if b.0.index() > 0 {
            true
        } else {
            false
        }
    }

    // TODO: this needs to be polymorphic over the num type. Add more later and
    // do that also all the useful functions are on Ptr<AnyTy> so I want Word to
    // be Ptr<NumTy> or Ptr<WordTy> but all the useful functions are defined on
    // Ptr<AnyTy>
    pub fn get_word(&self, n: Word) -> u32 {
        n.0.index()
    }
}

// TODO: unsure if I need implement the destructor for the compiler context
// impl<T> Drop for Context<T> {
//     fn drop(&mut self) {
//         unsafe {
//             let addr = self.base_ptr as *mut libc::c_void;
//             let size = self.capacity * std::mem::size_of::<T>();
//             munmap(addr, size);
//         }
//     }
// }
