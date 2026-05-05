use std::marker::PhantomData;

/// We wrap raw indices in Idx to type the index with a phantom type
#[repr(transparent)]
pub struct Idx<T> {
    pub idx: u32,
    _type: PhantomData<T>,
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Idx<T> {}
impl<T> std::fmt::Debug for Idx<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Idx({})", self.idx)
    }
}
impl<T> Idx<T> {
    pub fn new(idx: u32) -> Self {
        Self {
            idx,
            _type: PhantomData,
        }
    }
}

// same thing as a raw Idx but this time we need to keep a start and an offset
// to slice for vectors
#[derive(Copy, Clone, Debug)]
pub struct RangeIdx<T> {
    pub start: u32,
    pub length: u32,
    _type: PhantomData<T>,
}

impl<T> RangeIdx<T> {
    pub fn new(start: u32, length: u32) -> Self {
        Self {
            start,
            length,
            _type: PhantomData,
        }
    }
}

pub struct Arena<T> {
    base_ptr: *mut T,  // the backing memory
    next_index: usize, // next mem address to write to
    capacity: usize,
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
    pub fn get(&self, index: Idx<T>) -> &T {
        // Bounds checking is just a comparison against next_index [cite: 22]
        debug_assert!((index.idx as usize) < self.next_index);
        unsafe { &*self.base_ptr.add(index.idx as usize) }
    }

    pub fn get_range(&self, range: RangeIdx<T>) -> &[T] {
        let start = range.start as usize;
        let len = range.length as usize;

        debug_assert!(start + len <= self.next_index);
        unsafe {
            let start_ptr = self.base_ptr.add(start);
            std::slice::from_raw_parts(start_ptr, len)
        }
    }

    /// Allocates a value in the arena and returns its index.
    pub fn alloc(&mut self, value: T) -> Idx<T> {
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

        // 4. Increment the bump pointer for the next allocation
        self.next_index += 1;

        Idx::<T>::new(index)
    }
}
