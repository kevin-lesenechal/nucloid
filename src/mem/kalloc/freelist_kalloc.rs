/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::info;
use core::cmp::min;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{self, NonNull, copy_nonoverlapping};

use crate::misc::align_up;

const MIN_BLOCK_SIZE: usize = 8;
const BLOCK_MAGIC: u16 = 0xcafe;

pub struct FreelistAllocator<Backend: AllocatorBackend> {
    free_list: Option<NonNull<Block>>,
    last_block: Option<NonNull<Block>>,
    _marker: PhantomData<Backend>,
}

pub trait AllocatorBackend {
    fn new_pages(nr_pages: usize) -> Option<NonNull<()>>;
}

unsafe impl<B: AllocatorBackend> Send for FreelistAllocator<B> {}

#[repr(C, align(16))]
#[derive(Debug)]
struct Block {
    /// A pointer to the previous block, the one immediately before in memory,
    /// allocated or not. `None` for the first block.
    prev: Option<NonNull<Block>>,

    /// A pointer to the next block, the one immediately after in memory,
    /// allocated or not. `Non` for the last block.
    next: Option<NonNull<Block>>,

    /// A pointer to the next free block, `None` if this is the last free block.
    /// The block referenced must always be free. This is value is only valid
    /// for free blocks: allocated blocks have garbage value for this field.
    next_free: Option<NonNull<Block>>,

    /// The total number of bytes available inside the block past the block's
    /// header to be used for the user. This size is always a multiple of the
    /// header's alignment requirement (32 bytes), and can therefor exceed what
    /// the user requested.
    bsize: usize,

    /// A bitset of flags:
    ///   * `BLOCK_ALLOCATED_BIT`: whether the block is allocated or not;
    flags: u16,

    /// A magic value to check the validity of a block. Must be equal to
    /// `BLOCK_MAGIC`.
    magic: u16,

    _phantom: PhantomData<Block>,
}

impl Block {
    fn end_addr(&self) -> *const u8 {
        unsafe {
            (self as *const Block as *const u8)
                .add(size_of::<Self>() + self.bsize)
        }
    }

    fn as_user_ptr(&self) -> NonNull<u8> {
        unsafe {
            NonNull::new_unchecked((self as *const Block).add(1) as *mut u8)
        }
    }

    fn iter_prev(&self) -> BlockPrevIter {
        BlockPrevIter::new(self.into())
    }

    #[inline]
    const fn is_free(&self) -> bool {
        self.flags & BLOCK_ALLOCATED_BIT == 0
    }
}

const BLOCK_ALLOCATED_BIT: u16 = 0b0000_0001;

impl<Backend: AllocatorBackend> FreelistAllocator<Backend> {
    pub const fn new() -> Self {
        FreelistAllocator {
            free_list: None,
            last_block: None,
            _marker: PhantomData,
        }
    }

    pub unsafe fn alloc(&mut self, bsize: usize) -> Option<NonNull<u8>> {
        if bsize == 0 {
            return None;
        }

        let block = self.get_free_block(bsize)?;
        self.cut_free_block(block, bsize);
        self.mark_free_block_allocated(block);

        Some(unsafe { block.as_ref() }.as_user_ptr())
    }

    // TODO: do something smarter
    pub unsafe fn realloc(
        &mut self,
        ptr: *mut u8,
        bsize: usize,
    ) -> Option<NonNull<u8>> {
        if ptr.is_null() {
            return unsafe { self.alloc(bsize) };
        }

        let block = unsafe { &mut *(ptr as *mut Block).sub(1) };
        assert_eq!(
            block.magic, BLOCK_MAGIC,
            "kalloc: realloc(): invalid block magic, tried to realloc an invalid address"
        );
        assert!(!block.is_free(), "kalloc: realloc(): use-after-free");

        let new = unsafe { self.alloc(bsize)? };
        let copy_size = min(block.bsize, bsize);

        unsafe {
            copy_nonoverlapping(ptr, new.as_ptr(), copy_size);
        }

        Some(new)
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let block = unsafe { &mut *(ptr as *mut Block).sub(1) };
        assert_eq!(
            block.magic, BLOCK_MAGIC,
            "kalloc: dealloc(): invalid block magic, tried to free an invalid address"
        );
        assert!(!block.is_free(), "kalloc: dealloc(): double-free");

        let mut has_merged = false;

        // First, try to find a free block immediately after to extend into.
        if let Some(mut direct_next_free) =
            self.direct_next_free_block(block.into())
        {
            self.free_merge_to_left(block, unsafe {
                direct_next_free.as_mut()
            });

            has_merged = true;
        }

        // Try to find a free block immediately before to extend.
        if let Some(mut prev) = block.prev {
            let prev = unsafe { prev.as_mut() };
            if prev.is_free() {
                if let Some(mut next) = block.next {
                    unsafe { next.as_mut() }.prev = Some(prev.into());
                }
                prev.bsize += size_of::<Block>() + block.bsize;
                prev.next = block.next;
                prev.next_free = block.next_free;
                block.magic = 0xdead;
                return;
            }
        }

        if let Some(mut prev_free) = self.prev_free_block(block.into()) {
            let prev_free = unsafe { prev_free.as_mut() };
            block.next_free = prev_free.next_free;
            prev_free.next_free = Some(block.into());
        } else {
            if !has_merged {
                block.next_free = self.free_list;
            }
            self.free_list = Some(block.into());
        }

        block.flags &= !BLOCK_ALLOCATED_BIT;
    }

    /// Perform sanity check to ensure verifiable invariants are still valid.
    /// This is a valuable, albeit slow, function to call during development and
    /// testing to detect bugs and corruption. This will travers the link list
    /// and check nodes and their own links for discrepancies. Any issue
    /// detected will lead to a panic.
    pub fn self_check(&mut self) {
        let mut curr_block = self.last_block;
        let mut prev = None;

        while let Some(block) = curr_block {
            let block = unsafe { block.as_ref() };
            assert_eq!(
                block.magic, BLOCK_MAGIC,
                "block at {:?} has invalid magic value: {:?}",
                block as *const Block, block
            );
            assert_eq!(block.next, prev);
            assert!(block.bsize > 0);
            assert_eq!(block.bsize % align_of::<Block>(), 0);

            prev = Some(block.into());
            curr_block = block.prev;
        }

        let mut curr_block = self.free_list;

        while let Some(block) = curr_block {
            let block = unsafe { block.as_ref() };
            assert_eq!(
                block.magic, BLOCK_MAGIC,
                "block at {:?} has invalid magic value: {:?}",
                block as *const Block, block
            );
            assert!(block.bsize > 0);
            assert_eq!(block.bsize % align_of::<Block>(), 0);
            assert!(block.is_free());

            curr_block = block.next_free;
        }
    }

    fn free_merge_to_left(&mut self, left: &mut Block, right: &mut Block) {
        info!("entering");
        //self.self_check();
        assert!(!left.is_free());
        assert!(right.is_free());
        assert_eq!(left.next, Some(right.into()));

        let prev_free = self.prev_free_block(left.into());

        right.magic = 0xdead;
        left.bsize += right.bsize + size_of::<Block>();
        left.next = right.next;
        left.next_free = right.next_free;

        if let Some(mut right_next) = right.next {
            let right_next = unsafe { right_next.as_mut() };
            right_next.prev = Some(left.into());
        }

        if let Some(mut prev_free) = prev_free {
            let prev_free = unsafe { prev_free.as_mut() };
            prev_free.next_free = Some(left.into());
        } else {
            self.free_list = Some(left.into());
        }

        if let Some(last_block) = self.last_block {
            if right as *mut Block == last_block.as_ptr() {
                self.last_block = Some(left.into());
            }
        }

        left.flags &= !BLOCK_ALLOCATED_BIT;
        info!("leaving");
        //self.self_check();
        info!("left");
    }

    fn iter_free(&mut self) -> FreeBlockIter {
        FreeBlockIter::new(self.free_list)
    }

    fn get_free_block(&mut self, bsize: usize) -> Option<NonNull<Block>> {
        if let Some(free_block) = self.find_free_block(bsize) {
            return Some(free_block);
        }

        self.alloc_free_block(bsize)
    }

    fn find_free_block(&mut self, req_bsize: usize) -> Option<NonNull<Block>> {
        self.iter_free()
            .find(|&block| unsafe { block.as_ref() }.bsize >= req_bsize)
    }

    fn last_free_block(&mut self) -> Option<NonNull<Block>> {
        let mut block = unsafe { self.last_block?.as_mut() };

        while !block.is_free() {
            block = unsafe { block.prev?.as_mut() };
        }

        Some(block.into())
    }

    fn alloc_free_block(
        &mut self,
        mut user_size: usize,
    ) -> Option<NonNull<Block>> {
        user_size = align_up(user_size, align_of::<Block>());
        let ext_bsize = align_up(user_size + size_of::<Block>(), 4096);

        let block = unsafe {
            Backend::new_pages(ext_bsize >> 12)?.as_mut() as *mut ()
                as *mut Block
        };

        if let Some(mut last_free) = self.last_free_block() {
            let last_free = unsafe { last_free.as_mut() };

            if last_free.end_addr() == block as *const u8 {
                last_free.bsize += ext_bsize;
                return Some(last_free.into());
            }
        }

        let block = unsafe { &mut *block };
        block.prev = self.last_block; // FIXME: not always true
        block.next = None;
        block.next_free = None;
        block.bsize = ext_bsize - size_of::<Block>();
        block.flags = 0;
        block.magic = BLOCK_MAGIC;

        if let Some(mut prev) = block.prev {
            unsafe { prev.as_mut() }.next = Some(block.into());
        }

        let block_ptr = block.into();

        if let Some(mut last_free) = self.last_free_block() {
            let last_free = unsafe { last_free.as_mut() };
            assert!(last_free.next_free.is_none());
            last_free.next_free = Some(block_ptr);
        } else {
            self.free_list = Some(block_ptr);
        }

        // FIXME: just because we allocated a frame doesn't mean it's the last one
        self.last_block = Some(block_ptr);

        Some(block_ptr)
    }

    fn cut_free_block(&mut self, mut left_block: NonNull<Block>, bsize: usize) {
        let user_size = align_up(bsize, align_of::<Block>());

        let left_block = unsafe { left_block.as_mut() };
        assert!(
            user_size <= left_block.bsize,
            "the requested size exceeds the available space"
        );

        let ext_bsize_left = left_block.bsize - user_size;
        if ext_bsize_left < size_of::<Block>() + MIN_BLOCK_SIZE {
            return;
        }

        let bsize_left = ext_bsize_left - size_of::<Block>();
        let right_block = unsafe {
            &mut *((left_block as *mut Block as *mut u8)
                .add(size_of::<Block>() + user_size)
                as *mut Block)
        };

        right_block.prev = Some(left_block.into());
        right_block.next = left_block.next;
        right_block.next_free = left_block.next_free;
        right_block.bsize = bsize_left;
        right_block.flags = 0;
        right_block.magic = BLOCK_MAGIC;

        if let Some(mut left_next) = left_block.next {
            unsafe { left_next.as_mut() }.prev = Some(right_block.into());
        }

        left_block.next = Some(right_block.into());
        left_block.next_free = Some(right_block.into());
        left_block.bsize -= ext_bsize_left;

        let last_block = self
            .last_block
            .expect("there must be a last block if we are cutting one");
        if last_block.as_ptr() == left_block as *mut Block {
            self.last_block = Some(right_block.into());
        }
    }

    fn mark_free_block_allocated(&mut self, mut block: NonNull<Block>) {
        let prev_free = self.prev_free_block(block);
        let block = unsafe { block.as_mut() };

        if let Some(prev_free) = prev_free {
            unsafe { &mut *prev_free.as_ptr() }.next_free = block.next_free;
        } else {
            self.free_list = block.next_free;
        }

        block.flags |= BLOCK_ALLOCATED_BIT;
        block.next_free = None;
    }

    /// Return the first non-allocated block before `block`, `None` if there is
    /// no free block before.
    fn prev_free_block(
        &mut self,
        block: NonNull<Block>,
    ) -> Option<NonNull<Block>> {
        let first_free = self.free_list?;

        for prev in unsafe { block.as_ref() }.iter_prev().skip(1) {
            if unsafe { prev.as_ref() }.is_free() {
                return Some(prev);
            }
            if prev == first_free {
                break;
            }
        }

        None
    }

    /// Return the free block right after `block`, i.e. a block whose address
    /// is immediately after `block`; `None` if no such free block.
    fn direct_next_free_block(
        &mut self,
        block: NonNull<Block>,
    ) -> Option<NonNull<Block>> {
        let block = unsafe { block.as_ref() };
        let next = unsafe { block.next?.as_ref() };

        if next.is_free()
            && next as *const Block as *const u8 == block.end_addr()
        {
            Some(next.into())
        } else {
            None
        }
    }

    fn count_blocks(&mut self) -> usize {
        if let Some(last_block) = self.last_block {
            unsafe { last_block.as_ref() }.iter_prev().count()
        } else {
            0
        }
    }

    #[cfg(test)]
    fn debug_print_blocks(&mut self) {
        use crate::mem::VAddr;
        use crate::println;

        println!("free_list  = {:?}", self.free_list);
        println!("last_block = {:?}", self.last_block);
        if let Some(last_block) = self.last_block {
            for block in unsafe { last_block.as_ref() }.iter_prev() {
                let block = unsafe { block.as_ref() };
                println!(
                    "{}  {:?}  {:>8}  next={:?}  next_free={:?}",
                    if block.is_free() { "FREE" } else { "USED" },
                    VAddr::from(block as *const Block),
                    block.bsize,
                    block.next,
                    block.next_free
                );
            }
        }
    }
}

struct FreeBlockIter<'a> {
    curr_block: Option<ptr::NonNull<Block>>,
    _phantom: PhantomData<&'a Block>,
}

impl FreeBlockIter<'_> {
    pub fn new(first: Option<ptr::NonNull<Block>>) -> Self {
        Self {
            curr_block: first,
            _phantom: PhantomData,
        }
    }
}

impl<'a> Iterator for FreeBlockIter<'a> {
    type Item = NonNull<Block>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ptr) = self.curr_block {
            let block = unsafe { &*ptr.as_ptr() };
            debug_assert_eq!(
                block.magic, BLOCK_MAGIC,
                "found corrupted block while iterating"
            );
            debug_assert!(block.is_free(), "found non-free block in free list");
            self.curr_block = block.next_free;
            Some(ptr)
        } else {
            None
        }
    }
}

struct BlockPrevIter<'a> {
    curr_block: Option<NonNull<Block>>,
    _phantom: PhantomData<&'a Block>,
}

impl BlockPrevIter<'_> {
    pub fn new(first: NonNull<Block>) -> Self {
        Self {
            curr_block: Some(first),
            _phantom: PhantomData,
        }
    }
}

impl<'a> Iterator for BlockPrevIter<'a> {
    type Item = NonNull<Block>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ptr) = self.curr_block {
            let block = unsafe { ptr.as_ref() };
            debug_assert_eq!(
                block.magic, BLOCK_MAGIC,
                "found corrupted block ({:?}) while iterating",
                ptr
            );
            self.curr_block = block.prev;
            Some(ptr)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::test::export::mem::{MEMORY, MEMORY_MUTEX, reset_memory};
    use crate::arch::test::frame::reset_frame_allocator;
    use crate::mem::kalloc::FrameAllocatorBackend;
    use crate::mem::kalloc::freelist_kalloc::Block;
    use crate::mem::kalloc::freelist_kalloc::FreelistAllocator;
    use core::ptr::NonNull;
    use core::slice;

    type KernelAllocator = FreelistAllocator<FrameAllocatorBackend>;

    const BSZ: usize = core::mem::size_of::<Block>();

    #[test]
    fn it_allocates_one_block() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            let addr = do_alloc(&mut alloc, 10, BSZ);
            let slice = slice::from_raw_parts(addr.as_ptr(), 10);
            assert!(slice.iter().all(|&b| b == 0xf9));
            assert_eq!(alloc.count_blocks(), 2);
        }
    }

    #[test]
    fn it_allocates_three_blocks() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            let addr = do_alloc(&mut alloc, 1024 - BSZ, BSZ);
            let slice = slice::from_raw_parts(addr.as_ptr(), 1024 - BSZ);
            assert!(slice.iter().all(|&b| b == 0xf9));

            let addr = do_alloc(&mut alloc, 2048 - BSZ, BSZ + 1024);
            let slice = slice::from_raw_parts(addr.as_ptr(), 2048 - BSZ);
            assert!(slice.iter().all(|&b| b == 0xf9));

            let addr = do_alloc(&mut alloc, 1024 - BSZ, BSZ + 1024 + 2048);
            let slice = slice::from_raw_parts(addr.as_ptr(), 1024 - BSZ);
            assert!(slice.iter().all(|&b| b == 0xf9));

            assert_eq!(alloc.count_blocks(), 3);
        }
    }

    #[test]
    fn it_extends_the_trailing_free_block() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 3000, BSZ);
            let addr = do_alloc(&mut alloc, 3000, BSZ + 3008 + BSZ);
            let slice = slice::from_raw_parts(addr.as_ptr(), 3000);
            assert!(slice.iter().all(|&b| b == 0xf9));
        }
    }

    #[test]
    fn it_doesnt_extend_trailing_free_blocks_across_page_holes() {
        todo!()
    }

    #[test]
    fn it_deallocates_the_last_block() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 2048 - BSZ, BSZ);
            let addr = do_alloc(&mut alloc, 2048 - BSZ, BSZ + 2048);
            alloc.dealloc(addr.as_ptr());

            let addr = do_alloc(&mut alloc, 2048 - BSZ, BSZ + 2048);
            let slice = slice::from_raw_parts(addr.as_ptr(), 2048 - BSZ);
            assert!(slice.iter().all(|&b| b == 0xf9));

            assert_eq!(alloc.count_blocks(), 2);
        }
    }

    #[test]
    fn it_deallocates_middle_block() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 256, BSZ);
            let addr_mid = do_alloc(&mut alloc, 256, BSZ + 256 + BSZ);
            do_alloc(&mut alloc, 256, BSZ + 256 + BSZ + 256 + BSZ);
            alloc.dealloc(addr_mid.as_ptr());

            let addr = do_alloc(&mut alloc, 256, BSZ + 256 + BSZ);
            let slice = slice::from_raw_parts(addr.as_ptr(), 256);
            assert!(slice.iter().all(|&b| b == 0xf9));

            assert_eq!(alloc.count_blocks(), 4);
        }
    }

    #[test]
    fn it_chains_new_free_blocks() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            let first = do_alloc(&mut alloc, 2048 - BSZ, BSZ);
            do_alloc(&mut alloc, 2048 - BSZ, BSZ + 2048);
            alloc.dealloc(first.as_ptr());

            do_alloc(&mut alloc, 3000, 4096 + BSZ);
            assert_eq!(alloc.count_blocks(), 4);
        }
    }

    #[test]
    fn it_deallocates_and_merge_with_prev() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 256, 1 * BSZ + 0 * 256);
            let mid1 = do_alloc(&mut alloc, 256, 2 * BSZ + 1 * 256);
            let mid2 = do_alloc(&mut alloc, 256, 3 * BSZ + 2 * 256);
            do_alloc(&mut alloc, 256, 4 * BSZ + 3 * 256);
            assert_eq!(alloc.count_blocks(), 5);

            alloc.dealloc(mid1.as_ptr());
            alloc.dealloc(mid2.as_ptr());

            do_alloc(&mut alloc, 512 + BSZ, 2 * BSZ + 1 * 256);
            assert_eq!(alloc.count_blocks(), 4);
        }
    }

    #[test]
    fn it_deallocates_and_merge_with_next() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 256, 1 * BSZ + 0 * 256);
            let mid1 = do_alloc(&mut alloc, 256, 2 * BSZ + 1 * 256);
            let mid2 = do_alloc(&mut alloc, 256, 3 * BSZ + 2 * 256);
            do_alloc(&mut alloc, 256, 4 * BSZ + 3 * 256);
            assert_eq!(alloc.count_blocks(), 5);

            alloc.dealloc(mid2.as_ptr());
            alloc.dealloc(mid1.as_ptr());

            do_alloc(&mut alloc, 512 + BSZ, 2 * BSZ + 1 * 256);
            assert_eq!(alloc.count_blocks(), 4);
        }
    }

    #[test]
    fn it_deallocates_and_merge_with_prev_and_next() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            do_alloc(&mut alloc, 256, 1 * BSZ + 0 * 256);
            let mid1 = do_alloc(&mut alloc, 256, 2 * BSZ + 1 * 256);
            let mid2 = do_alloc(&mut alloc, 256, 3 * BSZ + 2 * 256);
            let mid3 = do_alloc(&mut alloc, 256, 4 * BSZ + 3 * 256);
            do_alloc(&mut alloc, 256, 5 * BSZ + 4 * 256);
            assert_eq!(alloc.count_blocks(), 6);

            alloc.self_check();
            alloc.dealloc(mid1.as_ptr());
            alloc.self_check();
            alloc.dealloc(mid3.as_ptr());
            alloc.self_check();
            alloc.dealloc(mid2.as_ptr());
            alloc.self_check();

            do_alloc(&mut alloc, 3 * 256 + 2 * BSZ, 2 * BSZ + 1 * 256);
            assert_eq!(alloc.count_blocks(), 4);
        }
    }

    #[test]
    fn it_allocates_after_a_free() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            let addr = alloc.alloc(0x05).unwrap();
            alloc.self_check();
            alloc.dealloc(addr.as_ptr());
            alloc.self_check();
            alloc.alloc(0x3000).unwrap();
            alloc.self_check();
        }
    }

    #[test]
    fn it_frees_all() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            let b1 = do_alloc(&mut alloc, 256, BSZ);
            let b2 = do_alloc(&mut alloc, 256, BSZ + 256 + BSZ);
            let b3 = do_alloc(&mut alloc, 256, BSZ + 256 + BSZ + 256 + BSZ);
            alloc.self_check();
            alloc.debug_print_blocks();
            alloc.dealloc(b1.as_ptr());
            alloc.debug_print_blocks();
            alloc.self_check();
            alloc.dealloc(b3.as_ptr());
            alloc.debug_print_blocks();
            alloc.self_check();
            alloc.dealloc(b2.as_ptr());
            alloc.self_check();
            assert_eq!(alloc.count_blocks(), 1);
        }
    }

    #[test]
    fn it_crashes() {
        let _lock = MEMORY_MUTEX.lock();
        reset_memory();
        reset_frame_allocator();

        let mut alloc = KernelAllocator::new();
        unsafe {
            alloc.alloc(0x25).unwrap();
            let addr = alloc.alloc(0x05).unwrap();
            alloc.self_check();
            alloc.dealloc(addr.as_ptr());
            alloc.self_check();
            alloc.alloc(0x19).unwrap();
            alloc.self_check();
        }
    }

    #[test]
    fn it_doesnt_merge_with_prev_across_page_holes() {
        unimplemented!()
    }

    #[test]
    fn it_doesnt_merge_with_next_across_page_holes() {
        unimplemented!()
    }

    fn do_alloc(
        alloc: &mut KernelAllocator,
        size: usize,
        exp_addr: usize,
    ) -> NonNull<u8> {
        unsafe {
            let addr = alloc.alloc(size).expect("couldn't allocate");
            alloc.self_check();
            assert_eq!(addr.as_ptr(), MEMORY.0.as_mut_ptr().add(exp_addr));
            addr
        }
    }
}
