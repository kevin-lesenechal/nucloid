/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::mem::{PAddr, get_va_size};
use crate::sync::Spinlock;
use crate::arch::mem::VA_BASE;

extern "C" {
    /// This is the PD3 (fourth entry of the PDPT) setup by `_start` (start.S)
    /// during the early boot process and mapping the higher-half virtual
    /// addresses of the kernel (starting at VA 0xc000_0000). Do not access this
    /// static directly, instead use the spinlock-guarded `KERNEL_PD`.
    #[link_name = "boot_pd3"]
    static mut _BOOT_PD3: PD;

    /// Early boot initialization in `_start` graciously provided us with 8 page
    /// tables mapping a total of 16 Mio of physical memory at VA 0xc000_0000.
    /// Do not access this static directly, instead use the spinlock-guarded
    /// `KERNEL_PD_PTS`.
    #[link_name = "boot_pd3_pt0"]
    static mut _BOOT_PD3_PTS: [PT; 8];

    /// The virtual address at which the kernel image, as loaded by the
    /// bootloader, resides; the address is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_start: u8;

    /// The address of the first byte past the kernel image in virtual memory.
    /// The address is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_end: u8;

    /// The numbers of bytes of the kernel image, including padding. The size
    /// is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_size: u8;

    static __kernel_text_start: u8;

    static __kernel_text_end: u8;

    static __kernel_rodata_start: u8;

    static __kernel_rodata_end: u8;
}

#[cfg(target_arch = "x86_64")]
const PDPT_ENTRY_COUNT: usize = 512;

#[cfg(target_arch = "x86")]
const PDPT_ENTRY_COUNT: usize = 4;

#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct PML4(pub [PML4Entry; 512]);

#[repr(C)]
pub struct PDPT(pub [PDPTEntry; PDPT_ENTRY_COUNT]);

#[repr(C)]
pub struct PD(pub [PDEntry; 512]);

#[repr(C)]
pub struct PT(pub [PTEntry; 512]);

impl PD {
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut PDEntry> {
        self.0.iter_mut()
    }
}

impl PT {
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut PTEntry> {
        self.0.iter_mut()
    }
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PML4Entry(pub u64);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PDPTEntry(pub u64);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PDEntry(pub u64);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PTEntry(pub u64);

static KERNEL_PD: Spinlock<&mut PD> = Spinlock::new(
    unsafe { &mut _BOOT_PD3 }
);

static KERNEL_PD_PTS: Spinlock<&mut [PT; 8]> = Spinlock::new(
    unsafe { &mut _BOOT_PD3_PTS }
);

#[repr(C)]
pub struct TableEntry(u64);

impl PDPTEntry {
    pub fn addr(&self) -> PAddr {
        PAddr(self.0 & 0x3fffffff_fffff000)
    }

    pub fn set_addr(&mut self, addr: PAddr) {
        assert_eq!(addr.0 & !0x3fffffff_fffff000, 0); // TODO: check reserved bits
        self.0 &= !0x3fffffff_fffff000;
        self.0 |= addr.0;
    }

    pub fn pd(&self) -> Option<*const PD> {
        if !self.is_present() {
            return None;
        }

        let pd_ptr = self.addr().into_vaddr()
            .expect("Couldn't map PDPTE's PD in VM")
            as *const PD;

        Some(pd_ptr)
    }

    pub fn pd_mut(&mut self) -> Option<*mut PD> {
        if !self.is_present() {
            return None;
        }

        let pd_ptr = self.addr().into_vaddr()
            .expect("Couldn't map PDPTE's PD in VM")
            as *mut PD;

        Some(pd_ptr)
    }

    pub fn is_present(&self) -> bool {
        self.0 & (1 << 0) > 0
    }

    pub fn set_present(&mut self, present: bool) {
        if present {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }
}

impl PDEntry {
    pub fn addr(&self) -> PAddr {
        PAddr(self.0 & 0x3fffffff_fffff000)
    }

    pub fn set_addr(&mut self, addr: PAddr) {
        assert_eq!(addr.0 & !0x3fffffff_fffff000, 0); // TODO: fix if huge
        self.0 &= !0x3fffffff_fffff000;
        self.0 |= addr.0;
    }

    pub fn pt(&self) -> Option<*const PT> {
        if !self.is_present() || self.is_huge() {
            return None;
        }

        let pt_ptr = self.addr().into_vaddr()
            .expect("Couldn't map PDE's PT in VM")
            as *const PT;

        Some(pt_ptr)
    }

    pub fn pt_mut(&mut self) -> Option<*mut PT> {
        if !self.is_present() || self.is_huge() {
            return None;
        }

        let pt_ptr = self.addr().into_vaddr()
            .expect("Couldn't map PDE's PT in VM")
            as *mut PT;

        Some(pt_ptr)
    }

    pub fn is_present(&self) -> bool {
        self.0 & (1 << 0) > 0
    }

    pub fn set_present(&mut self, present: bool) {
        if present {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }

    pub fn is_writable(&self) -> bool {
        self.0 & (1 << 1) > 0
    }

    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
    }

    pub fn is_huge(&self) -> bool {
        self.0 & (1 << 7) > 0
    }

    // TODO: panic if huge=true & addr is incorrect
    pub fn set_huge(&mut self, huge: bool) {
        if huge {
            self.0 |= 1 << 7;
        } else {
            self.0 &= !(1 << 7);
        }
    }

    pub fn is_executable(&self) -> bool {
        self.0 & (1 << 63) == 0
    }

    pub fn set_executable(&mut self, executable: bool) {
        if executable {
            self.0 &= !(1 << 63);
        } else {
            self.0 |= 1 << 63;
        }
    }
}

// TODO: remove code duplication
impl PTEntry {
    pub fn addr(&self) -> PAddr {
        PAddr(self.0 & 0x3fffffff_fffff000)
    }

    pub fn set_addr(&mut self, addr: PAddr) {
        assert_eq!(addr.0 & !0x3fffffff_fffff000, 0);
        self.0 &= !0x3fffffff_fffff000;
        self.0 |= addr.0;
    }

    pub fn is_present(&self) -> bool {
        self.0 & (1 << 0) > 0
    }

    pub fn set_present(&mut self, present: bool) {
        if present {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }

    pub fn is_writable(&self) -> bool {
        self.0 & (1 << 1) > 0
    }

    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
    }

    pub fn is_executable(&self) -> bool {
        self.0 & (1 << 63) == 0
    }

    pub fn set_executable(&mut self, executable: bool) {
        if executable {
            self.0 &= !(1 << 63);
        } else {
            self.0 |= 1 << 63;
        }
    }
}

pub enum AnyEntry {
    #[cfg(target_arch = "x86_64")]
    PML4Entry(PML4Entry),
    PDPTEntry(PDPTEntry),
    PDEntry(PDEntry),
    PTEntry(PTEntry),
}

pub fn locate_page_entry(vaddr: usize) -> Option<AnyEntry> {
    #[cfg(target_arch = "x86_64")]
    unimplemented!();

    let pdpt_index = vaddr >> 30; // FIXME: 32 bits only
    let pd_index = (vaddr & 0x3fe0_0000) >> 21;
    let pt_index = (vaddr & 0x001f_f000) >> 12;

    let pdpt_ptr = unsafe {
        PAddr((x86::controlregs::cr3() & 0xffff_f000) as u64)
            .into_vaddr()
            .expect("Couldn't map PDPT in VM")
            as *const PDPT
    };
    let pdpt = unsafe { &*pdpt_ptr };
    let pdpte = pdpt.0[pdpt_index];
    if !pdpte.is_present() {
        return None;
    }

    let pd = unsafe { &*pdpte.pd().unwrap() };
    let pde = pd.0[pd_index];
    if pde.is_huge() {
        return Some(AnyEntry::PDEntry(pde));
    } else if !pde.is_present() {
        return None;
    }

    let pt = unsafe { &*pde.pt().unwrap() };
    let pte = pt.0[pt_index];

    Some(AnyEntry::PTEntry(pte))
}

/// The paging configuration set up by `_start` in the early boot process is
/// enough to run code but requires some refinements. First, we need to protect
/// the kernel by making the .text and .rodata segments of its image read-only,
/// and also allowing execution exclusively within the .text segment.
///
/// After that, we continue to map physical memory past the kernel image up to
/// the maximum physical memory or the virtual address limit (up to 896 Mio),
/// whichever comes first. The mapped physical memory is readable and writable
/// but is not executable.
///
/// We take advantage of the 8 page-tables statically allocated within the
/// kernel image to bootstrap this process: in fact, mapping more virtual
/// memory requires the allocation of more page-tables, while allocating these
/// structures requires more virtual memory. Since creating a new page-table
/// gives us 2 Mio of additional virtual addresses for 4096 bytes for the table
/// itself, we only need a single spare memory page to map the entire virtual
/// address space.
///
/// # Return value
///
/// This function returns the virtual address of the first free byte right after
/// what this function used for new page-tables and where further allocations
/// can start.
///
/// # Side effects
///
/// After returning, up to `VA_SIZE` bytes of virtual memory are available and
/// identity-map the low physical memory as writable but not executable;
/// exceptions to this are the kernel's .text segment which is read-only and
/// executable, the kernel's .rodata segment which is read-only.
///
/// This function is the first one to write past the preallocated memory space
/// loaded by the bootloader (`__kernel_image_end`), to make new page-tables.
/// Any data located just after the kernel image are therefore overwritten which
/// notably includes the Multiboot structure provided by the bootloader; it is
/// thus vital to copy any needed information from this structure before
/// calling this function.
///
/// The entire TLB is invalidated.
///
/// # Safety
///
/// This function must only be called ONCE during the early-boot phase when SMP
/// is disabled and before any allocator or memory-dependent components are
/// initialized. This function assumes that the `__kernel_*` symbols passed by
/// the linker script are correct, and that the entire paging structure tree
/// starting at the fourth PDPT entry (as set up by `_start`) is valid.
/// After calling this function, the Multiboot information structure is invalid.
pub unsafe fn setup_kernel_paging() -> usize {
    let kernel_end = unsafe { &__kernel_image_end as *const u8 as usize };
    let text_start = unsafe { &__kernel_text_start as *const u8 as usize };
    let text_end = unsafe { &__kernel_text_end as *const u8 as usize };
    let rodata_start = unsafe { &__kernel_rodata_start as *const u8 as usize };
    let rodata_end = unsafe { &__kernel_rodata_end as *const u8 as usize };
    let boot_va_size = VA_BASE + _BOOT_PD3_PTS.len() * (2 << 20);

    let mut heap_addr = kernel_end;
    let mut vaddr: usize = VA_BASE;
    let mut kernel_pd = KERNEL_PD.lock();

    'each_pde: for pd_entry in kernel_pd.iter_mut() {
        if !pd_entry.is_present() {
            assert_eq!(heap_addr & 0xfff, 0);
            assert!(heap_addr + 4096 <= boot_va_size);
            let pt_ptr = heap_addr as *mut u8;
            unsafe {
                pt_ptr.write_bytes(0, 4096);
            }

            pd_entry.set_addr(
                PAddr::from_lowmem_vaddr(heap_addr)
                    .expect("Virtual address must be in low memory")
            );
            pd_entry.set_present(true);
            pd_entry.set_writable(true);

            heap_addr += 4096;
        }

        let pt = pd_entry.pt_mut().expect("PTE does not reference a PT");
        let pt = unsafe { &mut *pt };
        for pt_entry in pt.iter_mut() {
            if vaddr >= text_start && vaddr < text_end {
                pt_entry.set_writable(false);
            } else if vaddr >= rodata_start && vaddr < rodata_end {
                pt_entry.set_writable(false);
                pt_entry.set_executable(false);
            } else {
                pt_entry.set_executable(false);
            }

            vaddr += 4096;
            if vaddr >= get_va_size() {
                break 'each_pde;
            }
        }
    }

    assert_eq!(vaddr, get_va_size());

    unsafe {
        x86::controlregs::cr3_write(x86::controlregs::cr3());
    }

    heap_addr
}
