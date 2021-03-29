/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::ops::Deref;
use crate::mem::VAddr;
use crate::sync::Spinlock;
use crate::{debug, warning};
use crate::arch::mem::unmap_highmem_vaddr;

pub static HIGHMEM_ALLOCATOR: Spinlock<Option<HighmemAllocator>>
    = Spinlock::new(None);

pub struct HighmemAllocator {
    start: VAddr,
    nr_pages: usize,
    allocated: &'static mut [bool],
}

impl HighmemAllocator {
    pub unsafe fn new(start: VAddr,
                      nr_pages: usize,
                      buffer: &'static mut [bool]) -> Self {
        assert_eq!(buffer.len(), nr_pages);

        Self {
            start,
            nr_pages,
            allocated: buffer,
        }
    }

    /// Request the allocation of contiguous area of `nr_pages` virtual pages
    /// for high-memory mapping. No paging is actually performed, this merely
    /// return a virtual address that is reserved for the caller to map anything
    /// onto it.
    ///
    /// # Return #
    ///
    /// The virtual address of the first page; None if no such allocation is
    /// possible (high-memory space is exhausted for example). The `HighmemBox`
    /// smart pointer helps freeing the pages for latter use.
    pub fn allocate(&mut self, nr_pages: usize) -> Option<VAddr> {
        let mut nr_free: usize = 0;
        let mut free_index = None;

        for (i, &is_allocated) in self.allocated.iter().enumerate() {
            if !is_allocated {
                nr_free += 1;

                if nr_free == nr_pages {
                    free_index = Some(i - (nr_free - 1));
                    break;
                }
            } else {
                nr_free = 0;
            }
        }

        if let Some(free_index) = free_index {
            for i in free_index..(free_index + nr_pages) {
                self.allocated[i] = true;
            }
            let vaddr = self.start + free_index * 4096;
            debug!("allocated {nr_pages} high-memory pages starting at {vaddr:?}");
            return Some(vaddr);
        } else {
            warning!("no free high-memory addresses for {} pages", nr_pages);
            None
        }
    }

    /// Free previously allocated high-memory virtual addresses from the
    /// `allocate()` function. Once completed, the `nr_pages` pages starting at
    /// address `vaddr` will be made available for further high-memory
    /// allocations.
    ///
    /// # Safety #
    ///
    /// The caller must ensure that `vaddr` is a value given by the `allocate()`
    /// function that is currently valid: you must not free a non-high-memory
    /// area or free it twice. `nr_pages` must correspond to the number of pages
    /// used for allocation, you cannot partially free a high-memory area.
    /// Also, the caller must ensure that no paging mapping is currently in
    /// effect on any of the memory pages to free before freeing them. After
    /// free, the virtual address must not be used anymore unless given by a
    /// subsequent call to `allocate()`.
    pub unsafe fn free(&mut self, vaddr: VAddr, nr_pages: usize) {
        debug!("freed {nr_pages} high-memory pages starting at {vaddr:?}");

        let start_index = self.vaddr_to_index(vaddr);

        for allocated in self.allocated
            .iter_mut()
            .skip(start_index)
            .take(nr_pages) {
            *allocated = false;
        }
    }

    fn vaddr_to_index(&self, vaddr: VAddr) -> usize {
        assert_eq!(vaddr.0 & 0xfff, 0, "vaddr must be page-aligned");
        assert!(vaddr >= self.start
                && vaddr.0 <= self.start.0.saturating_add(self.nr_pages << 12),
                "vaddr is out of bound");

        (vaddr - self.start).0 >> 12
    }
}

pub struct HighmemGuard {
    addr: VAddr,
    nr_highmem_pages: usize,
}

impl HighmemGuard {
    pub unsafe fn new_allocated_highmem(addr: VAddr, nr_pages: usize) -> Self {
        Self {
            addr,
            nr_highmem_pages: nr_pages,
        }
    }

    pub fn new_lowmem(addr: VAddr) -> Self {
        Self {
            addr,
            nr_highmem_pages: 0
        }
    }

    pub fn unwrap_lowmem(self) -> VAddr {
        if self.is_highmem() {
            panic!("address {:?} is high-memory", self.addr);
        }

        self.addr
    }

    #[inline]
    pub fn is_highmem(&self) -> bool {
        self.nr_highmem_pages > 0
    }

    #[inline]
    pub fn is_lowmem(&self) -> bool {
        !self.is_highmem()
    }

    pub fn leak(self) -> VAddr {
        let addr = self.addr;
        core::mem::forget(self);

        addr
    }
}

impl Deref for HighmemGuard {
    type Target = VAddr;

    fn deref(&self) -> &Self::Target {
        &self.addr
    }
}

impl Drop for HighmemGuard {
    fn drop(&mut self) {
        if self.is_lowmem() {
            return;
        }

        for i in 0..self.nr_highmem_pages {
            unsafe {
                unmap_highmem_vaddr(self.addr + (i << 12));
            }
        }

        let mut allocator = HIGHMEM_ALLOCATOR.lock();
        unsafe {
            allocator.as_mut().unwrap().free(self.addr, self.nr_highmem_pages);
        }
    }
}

#[cfg(test)]
mod test {
    /*use crate::mem::highmem::HighmemAllocator;
    use crate::mem::VAddr;*/

    #[test]
    fn it_allocates_single_pages() {
        /*let mut buffer = [false; 8];
        let mut allocator = unsafe {
            HighmemAllocator::new(VAddr(0x1000), 8, &mut buffer)
        };

        allocator.allocate(1);*/
    }
}
