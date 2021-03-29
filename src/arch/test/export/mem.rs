use crate::mem::{PagePermissions, VAddr};
use crate::mem::highmem::HighmemGuard;
use crate::sync::Spinlock;

pub const FRAME_SIZE: usize = 4096;
pub const FRAME_SIZE_BITS: usize = 12;
pub const NR_PHYS_FRAMES: usize = 32;

#[repr(align(4096))]
pub struct VmMemory(pub [u8; NR_PHYS_FRAMES << 12]);

pub static mut MEMORY: VmMemory = VmMemory([0xf9; NR_PHYS_FRAMES << 12]);
pub static MEMORY_MUTEX: Spinlock<()> = Spinlock::new(());

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PAddr(pub u64);

impl PAddr {
    pub fn into_vaddr(self, _nr_pages: usize) -> Option<HighmemGuard> {
        unimplemented!()
    }

    pub fn into_lowmem_vaddr(self) -> Option<VAddr> {
        Some(VAddr(
            unsafe { MEMORY.0.get(self.0 as usize)? } as *const u8 as usize
        ))
    }

    pub fn from_lowmem_vaddr(_vaddr: usize) -> Option<PAddr> {
        unimplemented!()
    }

    pub fn is_highmem(&self) -> bool {
        false
    }
}

pub fn reset_memory() {
    unsafe { MEMORY.0.fill(0xf9); }
}

pub fn page_permissions(_vaddr: VAddr) -> PagePermissions {
    unimplemented!()
}

pub unsafe fn unmap_highmem_vaddr(_vaddr: VAddr) {
    unimplemented!()
}
