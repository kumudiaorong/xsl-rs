pub use allocator_api2::alloc::{handle_alloc_error, Allocator};
pub use allocator_api2::vec::Vec;

#[cfg(not(debug_assertions))]
pub use allocator_api2::alloc::Global;

#[cfg(debug_assertions)]
pub use inner::Global;

#[cfg(debug_assertions)]
mod inner {
    use allocator_api2::alloc::{AllocError, Allocator};
    use core::alloc::Layout;
    use core::ptr::NonNull;
    pub struct Global {
        count: usize,
        alloc: allocator_api2::alloc::Global,
    }
    impl Default for Global {
        fn default() -> Self {
            Self {
                count: 0,
                alloc: allocator_api2::alloc::Global,
            }
        }
    }
    impl Clone for Global {
        fn clone(&self) -> Self {
            Self {
                count: 0,
                alloc: self.alloc.clone(),
            }
        }
    }
    impl Drop for Global {
        fn drop(&mut self) {
            if self.count != 0 {
                panic!("Memory leak detected");
            }
        }
    }
    unsafe impl Allocator for Global {
        #[inline(always)]
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            unsafe {
                (&self.count as *const usize as *mut usize).write(self.count + 1);
            }
            self.alloc.allocate(layout)
        }

        #[inline(always)]
        fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            unsafe {
                (&self.count as *const usize as *mut usize).write(self.count + 1);
            }
            self.alloc.allocate_zeroed(layout)
        }

        #[inline(always)]
        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe {
                (&self.count as *const usize as *mut usize).write(self.count - 1);
            }
            self.alloc.deallocate(ptr, layout);
        }

        #[inline(always)]
        unsafe fn grow(
            &self,
            ptr: NonNull<u8>,
            old_layout: Layout,
            new_layout: Layout,
        ) -> Result<NonNull<[u8]>, AllocError> {
            self.alloc.grow(ptr, old_layout, new_layout)
        }

        #[inline(always)]
        unsafe fn grow_zeroed(
            &self,
            ptr: NonNull<u8>,
            old_layout: Layout,
            new_layout: Layout,
        ) -> Result<NonNull<[u8]>, AllocError> {
            self.alloc.grow_zeroed(ptr, old_layout, new_layout)
        }

        #[inline(always)]
        unsafe fn shrink(
            &self,
            ptr: NonNull<u8>,
            old_layout: Layout,
            new_layout: Layout,
        ) -> Result<NonNull<[u8]>, AllocError> {
            self.alloc.shrink(ptr, old_layout, new_layout)
        }
    }
}
