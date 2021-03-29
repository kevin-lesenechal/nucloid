/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt::{self, Debug, Formatter};

use crate::mem::{PagePermissions, get_lowmem_va_end, VAddr};
use crate::arch::x86::mem::paging::{locate_page_entry, AnyEntry, PD, reload_tlb, KERNEL_PDPT};
use crate::mem::highmem::HighmemGuard;

#[cfg(target_arch = "x86")]
use crate::mem::highmem::HIGHMEM_ALLOCATOR;

pub use crate::arch::x86::mem::paging::map_highmem_vaddr;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PAddr(pub u64);

impl Debug for PAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PA {:#016x}", self.0)
    }
}

impl PAddr {
    /// Convert the physical address into a virtual address. On x86-64, this
    /// always succeed
    #[cfg(target_arch = "x86_64")]
    // TODO: check paddr < phys_mem_size
    pub fn into_vaddr(self, _nr_pages: usize) -> Option<HighmemGuard> {
        let vaddr = VAddr(self.0 as usize) + LOWMEM_VA_START;
        Some(HighmemGuard::new_lowmem(vaddr))
    }

    /// Convert the physical address into a virtual address. On x86-32, this may
    /// require creating a high-memory allocation and mapping.
    #[cfg(target_arch = "x86")]
    // TODO: check paddr < phys_mem_size
    // BUG: don't reallocate highmem if already existent
    pub fn into_vaddr(self, nr_pages: usize) -> Option<HighmemGuard> {
        if self.is_highmem() {
            let mut highmem_alloc = HIGHMEM_ALLOCATOR.lock();
            let vaddr = highmem_alloc
                .as_mut()
                .expect("no high-memory allocator configured")
                .allocate(nr_pages)?;
            for i in 0..nr_pages {
                unsafe {
                    let page_off = i << 12;
                    map_highmem_vaddr(vaddr + page_off, self + page_off as u64);
                }
            }
            Some(unsafe {
                HighmemGuard::new_allocated_highmem(vaddr, nr_pages)
            })
        } else {
            let vaddr = VAddr(self.0 as usize) + LOWMEM_VA_START;
            Some(HighmemGuard::new_lowmem(vaddr))
        }
    }

    pub fn into_lowmem_vaddr(self) -> Option<VAddr> {
        if self.is_highmem() {
            None
        } else {
            Some(VAddr(self.0 as usize) + LOWMEM_VA_START)
        }
    }

    pub fn from_lowmem_vaddr(vaddr: VAddr) -> Option<PAddr> {
        if vaddr < LOWMEM_VA_START || vaddr >= get_lowmem_va_end() {
            None
        } else {
            Some(Self(vaddr.0 as u64 - LOWMEM_VA_START.0 as u64))
        }
    }

    #[cfg(target_arch = "x86_64")]
    pub const fn is_highmem(&self) -> bool {
        false
    }

    #[cfg(target_arch = "x86")]
    pub const fn is_highmem(&self) -> bool {
        self.0 >= LOWMEM_SIZE as u64
    }
}

impl VAddr {
    /// Retrieve the physical address at which this virtual address is mapped to
    /// if such mapping exists. This operation is rather costful since it
    /// requires traversing page tables.
    pub fn to_paddr(self) -> Option<PAddr> {
        Some(locate_page_entry(self)?.paddr())
    }

    #[cfg(target_arch = "x86_64")]
    pub fn pml4e(&self) -> usize {
        (self.0 & (0x1ff << 39)) >> 39
    }

    pub fn pdpte(&self) -> usize {
        (self.0 & (0x1ff << 30)) >> 30
    }

    pub fn pde(&self) -> usize {
        (self.0 & (0x1ff << 21)) >> 21
    }

    pub fn pte(&self) -> usize {
        (self.0 & (0x1ff << 12)) >> 12
    }

    pub fn pt_offset(&self) -> usize {
        self.0 & 0xfff
    }
}

/// The virtual address of the first byte of the low-memory area, i.e. the
/// virtual addresses that identity-map the physical address space.
#[cfg(target_arch = "x86_64")]
pub const LOWMEM_VA_START: VAddr = VAddr(0xffff8000_00000000);

#[cfg(target_arch = "x86_64")]
pub const LOWMEM_SIZE: usize = (128 << 40) - 1; // 128 Tio - 1

#[cfg(target_arch = "x86_64")]
pub const HIGHMEM_VA_SIZE: usize = 0;

#[cfg(target_arch = "x86")]
pub const LOWMEM_VA_START: VAddr = VAddr(0xc000_0000);

#[cfg(target_arch = "x86")]
pub const LOWMEM_SIZE: usize = 896 << 20; // 896 Mio

#[cfg(target_arch = "x86")]
pub const HIGHMEM_VA_SIZE: usize = 128 << 20; // 128 Mio

/// x86-32: 0xf800_0000
/// x86-64: (no high-memory)
pub const HIGHMEM_VA_START: VAddr = LOWMEM_VA_START + LOWMEM_SIZE;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SIZE_BITS: usize = 12;
pub const FRAME_SIZE: usize = 4096;
pub const FRAME_SIZE_BITS: usize = 12;

pub fn page_permissions(vaddr: VAddr) -> PagePermissions {
    let entry = locate_page_entry(vaddr);

    if entry.is_none() {
        return PagePermissions {
            accessible: false,
            readable: false,
            writable: false,
            executable: false,
        }
    }
    let entry = entry.unwrap();

    match entry {
        #[cfg(target_arch = "x86_64")]
        AnyEntry::PML4Entry(_) => unreachable!(),
        AnyEntry::PDPTEntry(_) => unimplemented!(),
        AnyEntry::PDEntry(pde) => PagePermissions {
            accessible: pde.is_present(),
            readable: pde.is_present(),
            writable: pde.is_present() && pde.is_writable(),
            executable: pde.is_present() && pde.is_executable(),
        },
        AnyEntry::PTEntry(pte) => PagePermissions {
            accessible: pte.is_present(),
            readable: pte.is_present(),
            writable: pte.is_present() && pte.is_writable(),
            executable: pte.is_present() && pte.is_executable(),
        }
    }
}

pub unsafe fn unmap_highmem_vaddr(vaddr: VAddr) {
    #[cfg(target_arch = "x86_64")] {
        assert_eq!(vaddr.pml4e(), 256,
                   "kernel virtual address must lie within PML4's 257th PDPT");
        let pdpt = KERNEL_PDPT.lock();
        let mut pdpte = pdpt.0[vaddr.pdpte()];

        if !pdpte.is_present() {
            panic!("kernel virtual address {:?} is not mapped", vaddr);
        }

        with_pd(vaddr, unsafe { &mut *pdpte.pd_mut().unwrap() });
    }

    #[cfg(target_arch = "x86")] {
        assert_eq!(vaddr.pdpte(), 3,
                   "kernel virtual address must list within PDPT's 4th PD");
        let mut pd = KERNEL_PD.lock();
        with_pd(vaddr, &mut pd);
    }

    fn with_pd(vaddr: VAddr, pd: &mut PD) {
        let pde = &mut pd.0[vaddr.pde()];

        if !pde.is_present() {
            panic!("kernel virtual address {:?} is not mapped", vaddr);
        }

        let pt = unsafe { &mut *pde.pt_mut().unwrap() };
        let pte = &mut pt.0[vaddr.pte()];

        if !pte.is_present() {
            panic!("kernel virtual address {:?} is not mapped", vaddr);
        }

        pte.set_present(false);

        // TODO: make more efficient by not trashing the entire TLB
        unsafe { reload_tlb(); }

        //debug!("unmapped {vaddr:?}");
    }
}
