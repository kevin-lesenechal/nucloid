/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

mod heap;

use crate::mem::kalloc::mimalloc::heap::Heap;
use crate::sync::Spinlock;
use crate::task::cpu::current_cpu_index;
use core::ptr::NonNull;

const SMALL_SIZE_MAX: usize = 1024;
const SMALL_SIZE_BUCKET_INC: usize = 8;
const SMALL_SIZE_BUCKET_INC_SHIFT: usize = 3;
const NR_DIRECT_PAGES: usize = SMALL_SIZE_MAX / SMALL_SIZE_BUCKET_INC;

#[repr(C)]
struct Segment {
    cpu_index: u8,
    magic: [u8; 3],
    page_shift: u32,
    pages: [PageHeader; 42],
}

pub struct PageHeader {
    prev: Option<NonNull<PageHeader>>,
    next: Option<NonNull<PageHeader>>,

    free_list: Option<NonNull<BlockHeader>>,
    deferred_free_list: Option<NonNull<BlockHeader>>,
    foreign_free_list: Spinlock<Option<NonNull<BlockHeader>>>,

    nr_block_used: usize,
}

enum PageAreaContainer {
    Small([u8; 42]),
}

pub struct BlockHeader {
    next: Option<NonNull<BlockHeader>>,
}

fn small_alloc(size: usize) -> NonNull<BlockHeader> {
    let cpu_index = current_cpu_index();
    let mut heap = Heap::for_cpu(&cpu_index).borrow_mut();

    heap.small_alloc(size)
}
