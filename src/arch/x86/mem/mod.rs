/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::mem::MaybeUninit;
use core::ptr::copy_nonoverlapping;
use arrayvec::ArrayVec;
use multiboot2::MemoryMapTag;

use crate::arch::x86::mem::paging::setup_kernel_paging;
use crate::arch::mem::{LOWMEM_VA_START, LOWMEM_SIZE};
use crate::mem::frame::{AllocatorBuilder, FRAME_ALLOCATOR};
use crate::mem::{PAddr, PHYS_MEM_SIZE};
use crate::debug;

#[cfg(target_arch = "x86")]
use crate::mem::highmem::{HIGHMEM_ALLOCATOR, HighmemAllocator};
#[cfg(target_arch = "x86")]
use crate::mem::frame::allocate_frames;
#[cfg(target_arch = "x86")]
use crate::misc::align_up;
use crate::misc::BinSize;

pub mod paging;
mod highmem;

pub fn lowmem_va_size(mem_maps: &MemoryMapTag) -> usize {
    let mut lowmem_size = 0;

    for area in mem_maps.all_memory_areas() {
        if area.start_address() >= LOWMEM_SIZE as u64 {
            break;
        } else if area.end_address() > LOWMEM_SIZE as u64 {
            return LOWMEM_SIZE;
        } else {
            lowmem_size = area.end_address() as usize;
        }
    }

    assert!(lowmem_size <= LOWMEM_SIZE);

    lowmem_size
}

pub fn physical_memory_size(mem_maps: &MemoryMapTag) -> u64 {
    mem_maps.all_memory_areas().map(|area| area.end_address()).max().unwrap()
}

pub unsafe fn boot_setup(mem_maps: &MemoryMapTag) {
    // We must first copy the array of memory area in the Multiboot struct that
    // will be destroyed by the call to `setup_kernel_paging()`.
    let mem_maps = copy_mbi_mem_areas(mem_maps);

    for area in mem_maps.iter() {
        debug!("[{}] {:?} -> {:?}    {:#10x} ({})",
               area.typ, PAddr(area.base_addr),
               PAddr(area.base_addr + area.length),
               area.length, BinSize(area.length));
    }

    let curr_heap = setup_kernel_paging();

    assert_eq!(curr_heap.0 & 0xfff, 0);
    let boot_used_bytes = (curr_heap - LOWMEM_VA_START).0 as u64;

    let mut allocator_b = AllocatorBuilder::new(curr_heap, PHYS_MEM_SIZE);

    for area in mem_maps {
        let paddr = PAddr(area.base_addr);
        let mut bsize = area.length;

        if paddr.0 == 0x9fc00 {
            continue;
        } else if paddr.0 == 0 {
            bsize &= !0xfff;
        }

        match area.typ {
            1 => {
                allocator_b.declare_unused_ram(paddr, bsize);
            },
            2 | 3 => {
                allocator_b.declare_reserved(paddr, bsize);
            },
            _ => {
                allocator_b.declare_unusable(paddr, bsize);
            }
        }
    }

    allocator_b.declare_allocated_ram(PAddr(0), boot_used_bytes);

    {
        let mut allocator = FRAME_ALLOCATOR.lock();
        assert!(allocator.is_none());
        *allocator = Some(allocator_b.build());
    }

    #[cfg(target_arch = "x86")] {
        use crate::arch::mem::{HIGHMEM_VA_START, HIGHMEM_VA_SIZE, PAGE_SIZE_BITS};

        let nr_highmem_pages = HIGHMEM_VA_SIZE >> PAGE_SIZE_BITS;
        let buffer = allocate_frames()
            .nr_frames(align_up(nr_highmem_pages, 4096) >> 12)
            .zero_mem()
            .map_lowmem()
            .expect("couldn't allocate high-memory buffer in low-memory")
            .as_mut_ptr::<bool>();
        let highmem_buff = core::slice::from_raw_parts_mut(
            buffer,
            nr_highmem_pages
        );

        let highmem_allocator = HighmemAllocator::new(
            HIGHMEM_VA_START,
            nr_highmem_pages,
            highmem_buff,
        );
        *HIGHMEM_ALLOCATOR.lock() = Some(highmem_allocator);
        debug!("High-memory allocator configured (VA {:?})",
               HIGHMEM_VA_START);
    }
}

fn copy_mbi_mem_areas(mem_maps: &MemoryMapTag) -> ArrayVec<MbiMemArea, 16> {
    let mut mem_areas: ArrayVec<MbiMemArea, 16> = ArrayVec::new();
    for area in mem_maps.all_memory_areas() {
        let mut area_copy = MaybeUninit::<MbiMemArea>::uninit();
        unsafe {
            copy_nonoverlapping(
                area as *const _ as *const u8,
                area_copy.as_mut_ptr() as *mut u8,
                core::mem::size_of::<MbiMemArea>(),
            );
        }
        mem_areas.push(unsafe { area_copy.assume_init() });
    }

    mem_areas
}

/// Exact copy of the multiboot2 crate's structure `MemoryArea`, since is does
/// not implement `Clone` nor can we access its fields, and we need to perm a
/// deep copy before trashing the MBI buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct MbiMemArea {
    base_addr: u64,
    length: u64,
    typ: u32,
    _reserved: u32,
}
