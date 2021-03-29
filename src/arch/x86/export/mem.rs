/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::mem::frame::{FrameAllocator, FRAME_ALLOCATOR};
use crate::mem::{PagePermissions, get_va_size, PHYS_MEM_SIZE};
use crate::arch::x86::mem::paging::{locate_page_entry, AnyEntry,
                                    setup_kernel_paging};
use crate::println;
use core::fmt::{self, Debug, Formatter};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PAddr(pub u64);

impl Debug for PAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PA {:#016x}", self.0)
    }
}

impl PAddr {
    #[cfg(target_arch = "x86_64")]
    // TODO: check paddr < phys_mem_size
    pub fn into_vaddr(self) -> Option<usize> {
        Some(self.0 + VA_BASE)
    }

    #[cfg(target_arch = "x86")]
    // TODO: check paddr < phys_mem_size
    pub fn into_vaddr(self) -> Option<usize> {
        if self.is_highmem() {
            None
        } else {
            Some(self.0 as usize + VA_BASE)
        }
    }

    pub fn from_lowmem_vaddr(vaddr: usize) -> Option<PAddr> {
        if vaddr < VA_BASE || vaddr >= get_va_size() {
            None
        } else {
            Some(Self(vaddr as u64 - VA_BASE as u64))
        }
    }

    pub fn is_highmem(&self) -> bool {
        self.0 >= LOWMEM_SIZE as u64
    }
}

#[cfg(target_arch = "x86_64")]
pub const VA_BASE: usize = unimplemented!();

#[cfg(target_arch = "x86")]
pub const VA_BASE: usize = 0xc000_0000;

pub const LOWMEM_SIZE: usize = 0x3800_0000; // 896 Mio
pub const HIGHMEM_VA_SIZE: usize = 0x0800_0000; // 128 Mio

/// x86-32: 0xf800_0000
/// x86-64: ???
pub const HIGHMEM_VA_START: usize = VA_BASE + LOWMEM_SIZE;

pub const FRAME_SIZE: usize = 4096;
pub const FRAME_SIZE_BITS: usize = 12;

pub unsafe fn boot_setup() {
    let curr_heap = setup_kernel_paging();

    assert_eq!(curr_heap & 0xfff, 0);
    let used_first_frames = (curr_heap - VA_BASE) >> 12;

    let mut allocator = FRAME_ALLOCATOR.lock();
    assert!(allocator.is_none());
    *allocator = Some(FrameAllocator::new(
        curr_heap as *mut (),
        PHYS_MEM_SIZE,
        used_first_frames
    ));
}

pub fn page_permissions(vaddr: usize) -> PagePermissions {
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
