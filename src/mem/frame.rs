/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::slice;
use core::mem::size_of;

use crate::sync::Spinlock;
use crate::mem::{PAddr, get_va_size};
use crate::arch::mem::{FRAME_SIZE, FRAME_SIZE_BITS};
use crate::misc::align_up;

#[derive(Copy, Clone)]
struct Frame {
    pub used: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Frame {
            used: false,
        }
    }
}

pub static FRAME_ALLOCATOR: Spinlock<Option<FrameAllocator>> = Spinlock::new(None);

pub struct FrameAllocator {
    frames: &'static mut [Frame],
}

impl FrameAllocator {
    pub unsafe fn new(buffer: *mut (),
                      phys_mem_size: u64,
                      used_first_frames: usize) -> Self {
        let num_frames = (align_up(phys_mem_size, 4096) >> 12) as usize;
        let buffer_size = num_frames * size_of::<Frame>();

        assert!(buffer as usize + buffer_size < get_va_size());
        let frames = unsafe {
            slice::from_raw_parts_mut(buffer as *mut Frame, num_frames)
        };
        frames.fill(Default::default());
        for frame in frames[0..used_first_frames].iter_mut() {
            frame.used = true;
        }

        Self {
            frames,
        }
    }

    pub fn allocate(&mut self, can_highmem: bool) -> Option<PAddr> {
        for (i, frame) in self.frames.iter_mut().enumerate() {
            if !frame.used {
                frame.used = true;
                return Some(Self::frame_paddr(i));
            }
        }

        None
    }

    pub fn allocate_map(&mut self,
                        can_highmem: bool,
                        zero: bool) -> Option<*mut ()> {
        let frame_addr = self.allocate(can_highmem)?;

        if frame_addr.is_highmem() {
            unimplemented!("Highmem is not implemented");
        }

        let ptr = frame_addr.into_vaddr().unwrap() as *mut u8;
        if zero {
            unsafe {
                ptr.write_bytes(0, 4096);
            }
        }

        Some(ptr as _)
    }

    pub fn free(&mut self, frame_addr: PAddr) {
        let index = Self::index_from_paddr(frame_addr);

        if index >= self.frames.len() {
            panic!("Free of out of bound frame at {}", frame_addr.0);
        }

        self.frames[index].used = false;
    }

    fn frame_paddr(frame_index: usize) -> PAddr {
        PAddr(frame_index as u64 * FRAME_SIZE as u64) // TODO: u64 non-portable
    }

    fn index_from_paddr(frame_paddr: PAddr) -> usize {
        (frame_paddr.0 >> FRAME_SIZE_BITS) as usize
    }
}
