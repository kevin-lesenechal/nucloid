use std::vec::Vec;
use crate::arch::test::export::mem::NR_PHYS_FRAMES;
use crate::mem::{VAddr, PAddr};
use crate::mem::frame::{AllocatorBuilder, Frame, FRAME_ALLOCATOR};
use crate::mem::LOWMEM_VA_END;

pub fn reset_frame_allocator() {
    unsafe { LOWMEM_VA_END = VAddr(usize::MAX); }

    let frames = Vec::<Frame>::with_capacity(NR_PHYS_FRAMES)
        .leak().as_mut_ptr();
    let mem_size = (NR_PHYS_FRAMES as u64) << 12;

    let allocator = unsafe {
        let mut a = AllocatorBuilder::new(frames.into(), mem_size);
        a.declare_unused_ram(PAddr(0), mem_size);
        a.build()
    };

    *FRAME_ALLOCATOR.lock() = Some(allocator);
}
