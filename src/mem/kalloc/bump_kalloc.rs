/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::mem::kalloc::freelist_kalloc::AllocatorBackend;
use crate::misc::align_up;

pub struct BumpAllocator<B> {
    heap_top: NonNull<()>,
    end_of_page: bool,
    _phantom: PhantomData<B>,
}

unsafe impl<B> Send for BumpAllocator<B> {} // TODO: justify

impl<B: AllocatorBackend> BumpAllocator<B> {
    pub const fn new() -> Self {
        Self {
            heap_top: unsafe { NonNull::new_unchecked(4096 as _) },
            end_of_page: false,
            _phantom: PhantomData,
        }
    }

    pub fn alloc(&mut self, bsize: usize) -> Option<NonNull<()>> {
        let mut block = unsafe {
            NonNull::new_unchecked(
                align_up(self.heap_top.as_ptr() as usize, 16) as *mut (),
            )
        };
        let bytes_left = align_up(self.heap_top.as_ptr() as usize, 4096)
            .saturating_sub(block.as_ptr() as usize);

        if bytes_left < bsize {
            let nr_pages = align_up(bsize, 4096) >> 12;
            block = B::new_pages(nr_pages)?;
        }

        self.heap_top = unsafe {
            NonNull::new_unchecked(
                (block.as_ptr() as *mut u8).add(bsize) as *mut ()
            )
        };

        Some(block)
    }

    pub unsafe fn dealloc(&mut self, _ptr: *mut ()) {}

    pub unsafe fn realloc(
        &mut self,
        ptr: *mut (),
        bsize: usize,
    ) -> Option<NonNull<()>> {
        if ptr.is_null() {
            return self.alloc(bsize);
        }

        unimplemented!()
        /*let new = self.alloc(bsize)?;
        let copy_size = min(block.bsize, bsize);

        unsafe {
            copy_nonoverlapping(ptr, new.as_ptr(), copy_size);
        }

        Some(new)*/
    }
}
