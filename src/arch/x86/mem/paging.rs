/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::mem::{PAddr, get_lowmem_va_end, VAddr};
use crate::sync::Spinlock;
use crate::arch::mem::LOWMEM_VA_START;
use crate::debug;
use crate::mem::load::{kernel_image, kernel_rodata_segment, kernel_text_segment};

extern "C" {
    #[link_name = "boot_pml4"]
    static mut _BOOT_PML4: PML4;

    #[link_name = "boot_pdpt256"]
    static mut _BOOT_PDPT256: PDPT;

    #[link_name = "boot_stack_bottom_guard"]
    /// The virtual address of first byte past the early boot kernel stack.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static boot_stack_bottom_guard: u8;
}

const PDPT_ENTRY_COUNT: usize = 512;

#[repr(C)]
pub struct PML4(pub [PML4Entry; 512]);

#[repr(C)]
pub struct PDPT(pub [PDPTEntry; PDPT_ENTRY_COUNT]);

#[repr(C)]
pub struct PD(pub [PDEntry; 512]);

#[repr(C)]
pub struct PT(pub [PTEntry; 512]);

impl PML4 {
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut PML4Entry> {
        self.0.iter_mut()
    }
}

impl PDPT {
    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut PDPTEntry> {
        self.0.iter_mut()
    }
}

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

static GLOBAL_PML4: Spinlock<&mut PML4> = Spinlock::new(
    unsafe { &mut _BOOT_PML4 }
);

pub(in crate::arch::x86) static KERNEL_PDPT: Spinlock<&mut PDPT>
    = Spinlock::new(unsafe { &mut _BOOT_PDPT256 });

#[repr(C)]
pub struct TableEntry(u64);

impl PML4Entry {
    pub fn addr(&self) -> PAddr {
        PAddr(self.0 & 0x3fffffff_fffff000)
    }

    pub fn set_addr(&mut self, addr: PAddr) {
        assert_eq!(addr.0 & !0x3fffffff_fffff000, 0); // TODO: check reserved bits
        self.0 &= !0x3fffffff_fffff000;
        self.0 |= addr.0;
    }

    pub fn pdpt(&self) -> Option<*const PDPT> {
        if !self.is_present() {
            return None;
        }

        Some(self.addr().into_vaddr().as_ptr())
    }

    pub fn pdpt_mut(&mut self) -> Option<*mut PDPT> {
        if !self.is_present() {
            return None;
        }

        Some(self.addr().into_vaddr().as_mut_ptr())
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

        Some(self.addr().into_vaddr().as_ptr())
    }

    pub fn pd_mut(&mut self) -> Option<*mut PD> {
        if !self.is_present() {
            return None;
        }

        Some(self.addr().into_vaddr().as_mut_ptr())
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

        Some(self.addr().into_vaddr().as_ptr())
    }

    pub fn pt_mut(&mut self) -> Option<*mut PT> {
        if !self.is_present() || self.is_huge() {
            return None;
        }

        Some(self.addr().into_vaddr().as_mut_ptr())
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

#[derive(Debug)]
pub enum AnyEntry {
    PML4Entry(PML4Entry),
    PDPTEntry(PDPTEntry),
    PDEntry(PDEntry),
    PTEntry(PTEntry),
}

impl AnyEntry {
    pub fn paddr(&self) -> PAddr {
        match self {
            AnyEntry::PML4Entry(e) => e.addr(),
            AnyEntry::PDPTEntry(e) => e.addr(),
            AnyEntry::PDEntry(e) => e.addr(),
            AnyEntry::PTEntry(e) => e.addr(),
        }
    }

    pub fn unwrap_pt_entry(self) -> PTEntry {
        match self {
            AnyEntry::PTEntry(e) => e,
            _ => panic!("AnyEntry is not a PTEntry"),
        }
    }
}

pub fn locate_page_entry(vaddr: VAddr) -> Option<AnyEntry> {
    let pdpt;
    let pdpt_index;

    let pml4_ptr = unsafe {
        PAddr(x86::controlregs::cr3() & 0x7fffffff_fffff000)
            .into_vaddr()
            .as_ptr::<PML4>()
    };
    let pml4 = unsafe { &*pml4_ptr };
    let pml4_index = (vaddr.0 & 0x0000ff80_00000000) >> 39;
    let pml4e = pml4.0[pml4_index];
    if !pml4e.is_present() {
        return None;
    }

    pdpt = unsafe { &*pml4e.pdpt().unwrap() };
    pdpt_index = (vaddr.0 & 0x0000007f_c0000000) >> 30;

    let pdpte = pdpt.0[pdpt_index];
    if !pdpte.is_present() {
        return None;
    }

    let pd = unsafe { &*pdpte.pd().unwrap() };
    let pd_index = (vaddr.0 & 0x3fe0_0000) >> 21;
    let pde = pd.0[pd_index];
    if pde.is_huge() {
        return Some(AnyEntry::PDEntry(pde));
    } else if !pde.is_present() {
        return None;
    }

    let pt = unsafe { &*pde.pt().unwrap() };
    let pt_index = (vaddr.0 & 0x001f_f000) >> 12;
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
pub unsafe fn setup_kernel_paging() -> VAddr {
    let mut heap_addr = kernel_image().end;
    let mut vaddr: VAddr = LOWMEM_VA_START;
    let mut pml4 = GLOBAL_PML4.lock();

    // Let's disable the bootstrapping PML4[0]
    pml4.0[0].set_present(false);

    'each_pml4e: for pml4_entry in pml4.iter_mut().skip(256) {
        if !pml4_entry.is_present() {
            unimplemented!();
        }

        let pdpt = pml4_entry.pdpt_mut().expect("No PDPT in PML4 entry");
        let pdpt = unsafe { &mut *pdpt };

        for pdpt_entry in pdpt.iter_mut() {
            if !pdpt_entry.is_present() {
                make_pd(pdpt_entry, &mut heap_addr);
            }

            let pd = pdpt_entry.pd_mut().expect("No PD in PDPT entry");
            let pd = unsafe { &mut *pd };

            walk_pd(pd, &mut heap_addr, &mut vaddr);
            if vaddr >= get_lowmem_va_end() {
                break 'each_pml4e;
            }
        }
    }

    assert_eq!(vaddr, get_lowmem_va_end());
    use crate::screen::R;
    debug!("Low memory mapped up to {:#x}", R(vaddr));

    unsafe {
        x86::controlregs::cr3_write(x86::controlregs::cr3());
    }

    heap_addr
}

fn walk_pd(pd: &mut PD, heap_addr: &mut VAddr, vaddr: &mut VAddr) {
    let text_segment = kernel_text_segment();
    let rodata_segment = kernel_rodata_segment();
    let stack_guard = VAddr(unsafe { &boot_stack_bottom_guard as *const u8 as usize });

    for pd_entry in pd.iter_mut() {
        if !pd_entry.is_present() {
            make_pt(pd_entry, heap_addr);
        }

        let pt = pd_entry.pt_mut().expect("PDE does not reference a PT");
        let pt = unsafe { &mut *pt };
        for pt_entry in pt.iter_mut() {
            let paddr = PAddr::from_lowmem_vaddr(*vaddr).unwrap();
            pt_entry.set_addr(paddr);
            pt_entry.set_present(true);

            if *vaddr == stack_guard {
                assert_eq!(unsafe { *vaddr.as_ptr::<u32>() }, 0xdeadbeef);
                pt_entry.set_present(false);
            } else if text_segment.contains(vaddr) {
                pt_entry.set_writable(false);
                pt_entry.set_executable(true);
            } else if rodata_segment.contains(vaddr) {
                pt_entry.set_writable(false);
                pt_entry.set_executable(false);
            } else {
                pt_entry.set_writable(true);
                pt_entry.set_executable(false);
            }

            *vaddr += 4096;
            if *vaddr >= get_lowmem_va_end() {
                return;
            }
        }
    }
}

// TODO: factorize with `make_pt()`
fn make_pd(pdpt_entry: &mut PDPTEntry, heap_addr: &mut VAddr) {
    assert_eq!(heap_addr.0 & 0xfff, 0);
    //assert!(*heap_addr + 4096 <= get_boot_lowmem_va_end());

    let pt_ptr = heap_addr.as_mut_ptr::<u8>();
    unsafe {
        pt_ptr.write_bytes(0, 4096);
    }

    pdpt_entry.set_addr(
        PAddr::from_lowmem_vaddr(*heap_addr)
            .expect("Virtual address must be in low memory")
    );
    pdpt_entry.set_present(true);
    pdpt_entry.set_writable(true);

    *heap_addr += 4096;

    // TODO: make more efficient by not trashing TLB at each new PD
    unsafe {
        x86::controlregs::cr3_write(x86::controlregs::cr3());
    }
}

fn make_pt(pd_entry: &mut PDEntry, heap_addr: &mut VAddr) {
    assert_eq!(heap_addr.0 & 0xfff, 0);
    //assert!(*heap_addr + 4096 <= get_boot_lowmem_va_end());

    let pt_ptr = heap_addr.as_mut_ptr::<u8>();
    unsafe {
        pt_ptr.write_bytes(0, 4096);
    }

    pd_entry.set_addr(
        PAddr::from_lowmem_vaddr(*heap_addr)
            .expect("Virtual address must be in low memory")
    );
    pd_entry.set_present(true);
    pd_entry.set_writable(true);

    *heap_addr += 4096;

    // TODO: make more efficient by not trashing TLB at each new PT
    unsafe { reload_tlb(); }
}

pub unsafe fn reload_tlb() {
    unsafe {
        x86::controlregs::cr3_write(x86::controlregs::cr3());
    }
}

fn get_boot_lowmem_va_end() -> VAddr {
    // 16 PDs are contained in the first 16 entries of PML4[256].PDPT
    unsafe { LOWMEM_VA_START + 16 * (2 << 20) }
}
