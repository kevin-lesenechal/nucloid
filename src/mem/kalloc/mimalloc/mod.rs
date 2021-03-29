mod heap;

use core::ptr::NonNull;
use crate::mem::kalloc::mimalloc::heap::Heap;
use crate::sync::Spinlock;
use crate::task::cpu::current_cpu_index;

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
