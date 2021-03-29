mod freelist_kalloc;
mod mimalloc;
mod bump_kalloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::ptr::NonNull;
use crate::error;
use crate::mem::frame::allocate_frames;
use crate::mem::kalloc::bump_kalloc::BumpAllocator;
use crate::mem::kalloc::freelist_kalloc::{AllocatorBackend};
use crate::sync::Spinlock;

pub struct KernelAllocatorWrapper(
    Spinlock<BumpAllocator<FrameAllocatorBackend>>
);

struct FrameAllocatorBackend;

impl AllocatorBackend for FrameAllocatorBackend {
    fn new_pages(nr_pages: usize) -> Option<NonNull<()>> {
        allocate_frames()
            .nr_frames(nr_pages)
            .map_lowmem()
            .map(|vaddr| NonNull::new(vaddr.as_mut_ptr()).unwrap())
    }
}

#[cfg_attr(not(test), global_allocator)]
pub static KERNEL_ALLOCATOR: KernelAllocatorWrapper
    = KernelAllocatorWrapper(Spinlock::new(BumpAllocator::new()));

unsafe impl GlobalAlloc for KernelAllocatorWrapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > 16 {
            error!("kernel allocator doesn't handle alignment requirements above 16 bytes");
            return ptr::null_mut();
        }

        self.0.lock().alloc(layout.size())
            .map(|p| p.as_ptr() as *mut u8)
            .unwrap_or(ptr::null_mut())
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.0.lock().dealloc(ptr as *mut ())
    }

    #[inline]
    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        _layout: Layout,
        new_size: usize
    ) -> *mut u8 {
        self.0.lock().realloc(ptr as *mut (), new_size)
            .map(|p| p.as_ptr() as *mut u8)
            .unwrap_or(ptr::null_mut())
    }
}

#[cfg(not(test))]
#[alloc_error_handler]
fn allocator_error(layout: Layout) -> ! {
    panic!("Kernel allocator failed: {:?}", layout)
}
