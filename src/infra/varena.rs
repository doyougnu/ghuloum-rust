use libc::{mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::ptr;

pub struct Arena<T> {
    base_ptr: *mut T,  // the backing memory
    next_index: usize, // next mem address to write to
    capacity: usize,
}

impl<T> Arena<T> {
    // TODO: split out the mmap from the arena
    // pub fn new(_base_ptr: *mut T, gb_to_reserve: usize) -> Self {
    pub fn new(gb_to_reserve: usize) -> Self {
        let capacity = gb_to_reserve / std::mem::size_of::<T>();
        let addr = unsafe {
            mmap(
                ptr::null_mut(),
                gb_to_reserve,
                PROT_READ | PROT_WRITE,      // we get read write permissions
                MAP_PRIVATE | MAP_ANONYMOUS, // PRIVATE + ANONYMOUS to reserve
                // the space then only get page
                // faults on a write (so we reserve
                // memory but don't write until
                // needed)
                -1,
                0,
            )
        };

        if addr == MAP_FAILED {
            panic!("Failed to reserve virtual memory. Check ulimit -v?");
        }

        Self {
            base_ptr: addr as *mut T,
            next_index: 0,
            capacity,
        }
    }

    #[inline(always)]
    pub fn get(&self, index: u32) -> &T {
        // Bounds checking is just a comparison against next_index [cite: 22]
        debug_assert!((index as usize) < self.next_index);
        unsafe { &*self.base_ptr.add(index as usize) }
    }

    /// Allocates a value in the arena and returns its index.
    pub fn alloc(&mut self, value: T) -> u32 {
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

        index
    }
}

impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        unsafe {
            let addr = self.base_ptr as *mut libc::c_void;
            let size = self.capacity * std::mem::size_of::<T>();
            munmap(addr, size);
        }
    }
}
