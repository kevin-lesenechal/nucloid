/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt::{self, Debug, Formatter};

use crate::mem::{PagePermissions, get_lowmem_va_end, VAddr};
use crate::arch::x86::mem::paging::{locate_page_entry, AnyEntry};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PAddr(pub u64);

impl Debug for PAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PA {:#016x}", self.0)
    }
}

impl PAddr {
    /// Convert the physical address into a virtual address.
    pub fn into_vaddr(self) -> VAddr {
        VAddr(self.0 as usize) + LOWMEM_VA_START
    }

    pub fn from_lowmem_vaddr(vaddr: VAddr) -> Option<PAddr> {
        if vaddr < LOWMEM_VA_START || vaddr >= get_lowmem_va_end() {
            None
        } else {
            Some(Self(vaddr.0 as u64 - LOWMEM_VA_START.0 as u64))
        }
    }
}

impl VAddr {
    /// Retrieve the physical address at which this virtual address is mapped to
    /// if such mapping exists. This operation is rather costful since it
    /// requires traversing page tables.
    pub fn to_paddr(self) -> Option<PAddr> {
        Some(locate_page_entry(self)?.paddr())
    }

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
pub const LOWMEM_VA_START: VAddr = VAddr(0xffff8000_00000000);

pub const LOWMEM_SIZE: usize = (128 << 40) - 1; // 128 Tio - 1

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
