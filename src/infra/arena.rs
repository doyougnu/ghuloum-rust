use std::marker::PhantomData;

/***************************** Pointer Types **********************************/
/// We wrap raw indices in Ptr to type the index with a phantom type
#[repr(transparent)]
pub struct Ptr<T> {
    pub idx: u32,
    _type: PhantomData<T>,
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Ptr<T> {}
impl<T> std::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ptr({})", self.idx)
    }
}
impl<T> std::ops::BitAnd<u32> for Ptr<T> {
    type Output = u32;
    fn bitand(self, rhs: u32) -> Self::Output {
        self.idx & rhs
    }
}
impl<T> std::cmp::PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> std::cmp::Eq for Ptr<T> {}

impl<T> std::cmp::PartialOrd for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}

// our address space defines a total order
impl<T> std::cmp::Ord for Ptr<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl<T> Ptr<T> {
    pub fn new(idx: u32) -> Self {
        Self {
            idx,
            _type: PhantomData,
        }
    }
    pub fn cast<NewT>(self) -> Ptr<NewT> {
        Ptr {
            idx: self.idx,
            _type: PhantomData,
        }
    }
}

/// A slice pointer type
#[derive(Copy, Clone, Debug)]
pub struct Slice<Typ> {
    pub start: usize,
    pub length: usize,
    _type: PhantomData<Typ>,
}

impl<T> Into<usize> for Ptr<T> {
    fn into(self) -> usize {
        self.idx as usize
    }
}

impl<Typ> Slice<Typ> {
    pub fn new(start: usize, length: usize) -> Self {
        Self {
            start,
            length,
            _type: PhantomData,
        }
    }

    // 0-cost casting of the phantom type. Needed to update the type tags in
    // arenas on a range_alloc
    pub fn cast<NewTyp>(self) -> Slice<NewTyp> {
        Slice {
            start: self.start,
            length: self.length,
            _type: PhantomData,
        }
    }
}

/****************************** The Arena **************************************/
pub struct Arena<T> {
    base_ptr: *mut T,  // the backing memory
    next_index: usize, // next mem address to write to
    capacity: usize,   // TODO: think about capacity
}

impl<T> Arena<T> {
    pub fn new(base: *mut T, gb_to_reserve: usize) -> Self {
        let capacity = gb_to_reserve / std::mem::size_of::<T>();

        Self {
            base_ptr: base,
            next_index: 0,
            capacity,
        }
    }

    #[inline(always)]
    pub fn get(&self, index: Ptr<T>) -> &T {
        unsafe { &*self.base_ptr.add(index.into()) }
    }

    pub fn get_range(&self, range: Slice<T>) -> &[T] {
        let start = range.start;
        let len = range.length;

        debug_assert!(start + len <= self.next_index);
        unsafe {
            let start_ptr = self.base_ptr.add(start);
            std::slice::from_raw_parts(start_ptr, len)
        }
    }
    /// Allocates a value in the arena and returns its index.
    pub fn alloc(&mut self, value: T) -> Ptr<T> {
        if self.next_index >= self.capacity {
            panic!("Arena capacity exceeded!");
        }
        let index = self.next_index as u32;

        unsafe {
            // Calculate the destination address using pointer arithmetic This
            // is base_ptr + (next_index * size_of::<T>()), add uses <T> to
            // implicitly call sizeof
            let slot_ptr = self.base_ptr.add(self.next_index);

            // Write the value to the backing memory. Using ptr::write is safer
            // than dereferencing (*slot_ptr = value) because it handles
            // uninitialized memory correctly.
            std::ptr::write(slot_ptr, value);
        }

        self.next_index += 1;

        Ptr::<T>::new(index)
    }

    pub fn alloc_range(&mut self, slice: &[T]) -> Slice<T> {
        let len = slice.len();
        if self.next_index + len >= self.capacity {
            // TODO: need to check the entire range
            panic!("alloc_range: Arena capacity exceeded");
        }
        let start = self.next_index;
        unsafe {
            let dest = self.base_ptr.add(start);
            std::ptr::copy_nonoverlapping(slice.as_ptr(), dest, len);
        }

        self.next_index += len;

        Slice::new(
            start
                .try_into()
                .unwrap_or_else(|_| panic!("Index too large for Size type")),
            len.try_into()
                .unwrap_or_else(|_| panic!("Length too large for Size type")),
        )
    }
}
