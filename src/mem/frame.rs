/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

///! Physical memory's frames management. Physical memory is divided into
///! continuous fixed-size units called *frames*. It is the most basic unit the
///! kernel uses to handle physical memory management and allocations.
///!
///! This module contains the definition of a frame and the *frame allocator*
///! which manages a global array of frames mapping the entire physical address
///! space.

use core::slice;
use core::mem::size_of;

use crate::sync::Spinlock;
use crate::mem::{PAddr, get_lowmem_va_end, VAddr};
use crate::arch::mem::{FRAME_SIZE, FRAME_SIZE_BITS};
use crate::{debug, error};
use crate::mem::highmem::HighmemGuard;
use crate::misc::align_up;

#[derive(Debug, Copy, Clone)]
pub struct Frame {
    state: FrameState,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum FrameState {
    /// The memory frame cannot be used for any usage.
    Unusable,

    /// The physical memory frame is free for use, any kind of allocate may use
    /// it for general purpose. Only frames in this state can be allocated
    /// through general `allocate()` call of the frame allocator.
    FreeRAM,

    /// This frame is currently in used.
    AllocatedRAM,

    /// Special memory area but currently unused. Those are generally MMIO
    /// devices like PCI BARs, framebuffers, etc. Frames marked as reserved
    /// won't be returned by a general-purpose allocation and must be
    /// specifically claimed through a `claim()` call on the frame allocator.
    /// At boot, the kernel will mark as reserved all frames as stated by
    /// the bootloader.
    UnclaimedReserved,

    ClaimedReserved,
}

impl Frame {
    fn is_allocated(&self) -> bool {
        matches!(self.state, FrameState::AllocatedRAM)
    }

    fn is_free_ram(&self) -> bool {
        matches!(self.state, FrameState::FreeRAM)
    }

    fn is_unusable(&self) -> bool {
        matches!(self.state, FrameState::Unusable)
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame {
            state: FrameState::Unusable,
        }
    }
}

//----------------------------------------------------------------------------//

pub static FRAME_ALLOCATOR: Spinlock<Option<FrameAllocator>> = Spinlock::new(None);

pub struct FrameAllocator {
    frames: &'static mut [Frame],
}

impl FrameAllocator {
    /// Allocate a single frame from general purpose RAM. No particular virtual
    /// memory mapping is performed, it is up to the caller to setup such VM
    /// mappings to access the allocated frame.
    ///
    /// # Parameters #
    ///
    /// * `can_highmem`: specifies whether the caller allows the allocator to
    ///                  reserve a frame in high-memory, the call will fail if
    ///                  this parameter is false (the caller refuses high-memory
    ///                  frames) and no low-memory frame is available.
    ///
    /// # Return #
    ///
    /// The physical address of the allocated frame's first byte, None if no
    /// frame could be found that satisfies the request.
    // BUG: strongly prefer high-memory if `can_highmem`
    pub fn allocate(
        &mut self,
        nr_frames: usize,
        can_highmem: bool,
    ) -> Option<PAddr> {
        let mut nr_free = 0;
        let mut free_index = None;

        for (i, frame) in self.frames.iter_mut().enumerate() {
            if frame.is_free_ram() {
                let paddr = Self::frame_paddr(i);
                if paddr.is_highmem() && !can_highmem {
                    return None;
                }
                nr_free += 1;

                if nr_free == nr_frames {
                    free_index = Some(i - (nr_free - 1));
                    break;
                }
            } else {
                nr_free = 0;
            }
        }

        if let Some(free_index) = free_index {
            for frame in self.frames
                .iter_mut()
                .skip(free_index)
                .take(nr_frames) {
                frame.state = FrameState::AllocatedRAM;
            }

            Some(Self::frame_paddr(free_index))
        } else {
            None
        }
    }

    /// Allocate a single frame from general purpose RAM and create a writable
    /// virtual memory mapping to it. Optionally, zero the page.
    ///
    /// If successful, the caller owns the page's memory area up until it is
    /// deallocated; the allocator guarantees that no other references will
    /// point in the page addresses, so the caller may safely create a mutable
    /// reference to it.
    ///
    /// # Parameters #
    ///
    /// * `can_highmem`: specifies whether the caller allows the allocator to
    ///                  reserve a frame in high-memory, the call will fail if
    ///                  this parameter is false (the caller refuses high-memory
    ///                  frames) and no low-memory frame is available;
    /// * `zero`: whether to initialize the frame with zeroes or not.
    ///
    /// # Return #
    ///
    /// The virtual address at which the page mapping the frame in VM resides,
    /// `None` if no frame could be found that satisfies the request.
    pub fn allocate_map(&mut self,
                        _nr_frames: usize,
                        _can_highmem: bool,
                        _zero: bool) -> Option<HighmemGuard> {
        panic!("deprecated")
    }

    pub unsafe fn free(&mut self, frame_addr: PAddr, nr_frames: usize) {
        assert_eq!(nr_frames, 1, "unimplemented");
        let index = Self::index_from_paddr(frame_addr);

        if index >= self.frames.len() {
            panic!("Free of out of bound frame at {}", frame_addr.0);
        }

        let new_state = match self.frames[index].state {
            FrameState::AllocatedRAM => FrameState::FreeRAM,
            FrameState::ClaimedReserved => FrameState::UnclaimedReserved,
            _ => panic!("trying to free unallocated frame"),
        };
        self.frames[index].state = new_state;
    }

    fn frame_paddr(frame_index: usize) -> PAddr {
        PAddr(frame_index as u64 * FRAME_SIZE as u64) // TODO: u64 non-portable
    }

    fn index_from_paddr(frame_paddr: PAddr) -> usize {
        (frame_paddr.0 >> FRAME_SIZE_BITS) as usize
    }
}

pub fn allocate_frames() -> AllocationBuilder {
    AllocationBuilder {
        nr_frames: 1,
        zero: false,
        can_highmem: false,
    }
}

pub struct AllocationBuilder {
    nr_frames: usize,
    zero: bool,
    can_highmem: bool,
}

impl AllocationBuilder {
    pub fn nr_frames(&mut self, nr_frames: usize) -> &mut Self {
        self.nr_frames = nr_frames;
        self
    }

    pub fn allow_highmem(&mut self) -> &mut Self {
        self.can_highmem = true;
        self
    }

    pub fn zero_mem(&mut self) -> &mut Self {
        self.zero = true;
        self
    }

    pub fn allocate(&mut self) -> Option<PAddr> {
        let mut allocator = FRAME_ALLOCATOR.lock();

        let paddr = allocator
            .as_mut()
            .expect("no frame allocator configured")
            .allocate(self.nr_frames, self.can_highmem)?;

        if self.zero {
            if let Some(vaddr) = paddr.into_vaddr(self.nr_frames) {
                unsafe {
                    vaddr.as_mut_ptr::<u8>()
                        .write_bytes(0, self.nr_frames * 4096);
                }
            } else {
                error!("couldn't zero frame: failed to map in high-memory");
                unsafe {
                    allocator
                        .as_mut()
                        .expect("no frame allocator configured")
                        .free(paddr, self.nr_frames);
                }
            }
        }

        Some(paddr)
    }

    pub fn map_lowmem(&mut self) -> Option<VAddr> {
        let mut allocator = FRAME_ALLOCATOR.lock();

        let vaddr = allocator
            .as_mut()
            .expect("no frame allocator configured")
            .allocate(self.nr_frames, self.can_highmem)?
            .into_lowmem_vaddr()?;

        if self.zero {
            unsafe {
                vaddr.as_mut_ptr::<u8>().write_bytes(0, self.nr_frames * 4096);
            }
        }

        Some(vaddr)
    }
}

//----------------------------------------------------------------------------//

pub struct AllocatorBuilder {
    frames: &'static mut [Frame],
}

impl AllocatorBuilder {
    /// Create a new frame allocator builder. This function is given a memory
    /// area through `buffer` that will be used for the global array of frames;
    /// it is called once during early boot process and forms the basis for
    /// memory allocation in the kernel. It is up to the early boot process to
    /// reserve a `buffer` big enough to map the entire physical address space.
    ///
    /// The number of frames instances is determined through the `phys_mem_size`
    /// parameter: it is equal to `phys_mem_size / 4096` and rounded up.
    ///
    /// # Parameters #
    ///
    /// * `buffer`: a writable memory area to use for the kernel's global array
    ///             of frames;
    /// * `phys_mem_size`: the size in bytes of all available physical memory.
    ///
    /// # Safety #
    ///
    /// `frame_array` must contain the virtual address to a writable memory area
    /// whose size is enough to accommodate for all required frames; the frame
    /// allocator will take ownership of this area by creating a mutable
    /// reference: the caller must guarantee that no reference will continue to
    /// point to it.
    ///
    /// After creation, the caller must declare via `declare_allocated()` all
    /// memory areas already in use, which includes the `buffer` used for the
    /// global array of frames.
    pub unsafe fn new(
        frame_array: VAddr,
        phys_mem_bsize: u64,
    ) -> AllocatorBuilder {
        let nr_frames = (align_up(phys_mem_bsize, 4096) >> 12) as usize;
        let array_bsize = nr_frames * size_of::<Frame>();

        assert!(frame_array + array_bsize < get_lowmem_va_end());
        let frames = unsafe {
            slice::from_raw_parts_mut(frame_array.as_mut_ptr(), nr_frames)
        };
        frames.fill(Default::default());

        Self {
            frames,
        }
    }

    /// Declare some physical memory area as already allocated and in use for
    /// general purpose allocations. This function is used when creating the
    /// allocator service to declare which memory areas were already in use for
    /// the kernel image and memory manually allocated by the paging system.
    /// It should not be used outside of the booting process.
    ///
    /// # Parameters #
    ///
    /// * paddr: the physical address where the memory area to set as allocated
    ///          starts, must be page-aligned;
    /// * bsize: the number of bytes that the memory spans, must be a multiple of
    ///          page size;
    ///
    /// # Safety #
    ///
    /// The caller must ensure that the covered area only include general-
    /// purpose RAM and no reserved physical memory (e.g. MMIO, PCI BARs, ...).
    ///
    /// # Panics #
    ///
    /// Panics if parameters' values are not properly aligned; or if the given
    /// size goes beyond the declared physical memory size.
    pub unsafe fn declare_allocated_ram(&mut self, paddr: PAddr, bsize: u64) {
        debug!("Declared RAM used {paddr:?} -> {:?}", paddr + bsize);
        self.set_state(paddr, bsize, FrameState::AllocatedRAM);
    }

    pub unsafe fn declare_unused_ram(&mut self, paddr: PAddr, bsize: u64) {
        self.set_state(paddr, bsize, FrameState::FreeRAM);
    }

    pub unsafe fn declare_reserved(&mut self, paddr: PAddr, bsize: u64) {
        self.set_state(paddr, bsize, FrameState::UnclaimedReserved);
    }

    pub unsafe fn declare_unusable(&mut self, paddr: PAddr, bsize: u64) {
        self.set_state(paddr, bsize, FrameState::Unusable);
    }

    /// Finish the allocator building and return the configured allocator.
    ///
    /// # Safety #
    ///
    /// The caller must ensure that all RAM areas already in use up until now
    /// were declared via `declare_allocated_ram` and that all reserved memory
    /// areas (e.g. MMIO, PCI BARs, ...) were declared via `declare_reserved`.
    /// Failure to do so will either hand already-allocated frames to other
    /// users, or allocate reserved memory areas for general purpose.
    pub unsafe fn build(mut self) -> FrameAllocator {
        let frames_paddr = PAddr::from_lowmem_vaddr(VAddr::from(self.frames.as_ptr())).unwrap();
        let frames_bsize = self.frames.len() * size_of::<Frame>();

        // Let's not forget to mark as used the RAM for the frame descriptors.
        self.declare_allocated_ram(
            frames_paddr,
            align_up(frames_bsize as u64, 4096)
        );

        FrameAllocator {
            frames: self.frames,
        }
    }

    fn set_state(&mut self, paddr: PAddr, bsize: u64, state: FrameState) {
        assert_eq!(paddr.0 & 0xfff, 0, "frame address is not 4 Kio-aligned");
        assert_eq!(bsize & 0xfff, 0, "frame size is not a multiple of 4 Kio");

        let index = FrameAllocator::index_from_paddr(paddr);
        let nr_frames = (bsize >> 12) as usize;

        for frame in self.frames[index..(index + nr_frames)].iter_mut() {
            frame.state = state;
        }
    }
}

//----------------------------------------------------------------------------//

#[cfg(test)]
mod test {
}
