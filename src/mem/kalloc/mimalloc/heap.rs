use core::cell::RefCell;
use core::mem::{MaybeUninit, size_of};
use core::ptr::NonNull;
use crate::arch::sync::{push_critical_region, pop_critical_region};
use crate::mem::kalloc::mimalloc::{BlockHeader, NR_DIRECT_PAGES, PageHeader, SMALL_SIZE_BUCKET_INC, SMALL_SIZE_BUCKET_INC_SHIFT};
use crate::misc::{align_up, first_bit_pos};
use crate::task::cpu::{CpuIndex, MAX_CPUS};
use crate::task::cpu_local::CpuLocal;

static HEAPS: CpuLocal<RefCell<Heap>>
    = CpuLocal::new(Heap::new_dangling_array());

#[derive(Copy, Clone)]
pub struct Heap {
    pub cpu_index: u8,
    pub direct_pages: [NonNull<RefCell<PageHeader>>; NR_DIRECT_PAGES],
    pub pages_list: [NonNull<RefCell<PageHeader>>; NR_DIRECT_PAGES],
}

impl Heap {
    const fn new_dangling() -> Self {
        Self {
            cpu_index: 0,
            direct_pages: [NonNull::dangling(); NR_DIRECT_PAGES],
            pages_list: [NonNull::dangling(); NR_DIRECT_PAGES],
        }
    }

    const fn new_dangling_array() -> [RefCell<Self>; MAX_CPUS] {
        let mut arr = MaybeUninit::<[RefCell<Self>; MAX_CPUS]>::uninit();

        let mut i = 0;
        while i < MAX_CPUS {
            let arr_ref = unsafe { &mut *arr.as_mut_ptr() };
            arr_ref[i] = RefCell::new(Heap::new_dangling());
            i += 1;
        }

        unsafe { arr.assume_init() }
    }

    unsafe fn init(&mut self, cpu_index: u8) {
        self.cpu_index = cpu_index;
    }

    /// Proceed to initialize all per-CPU heap. This function must be called
    /// once during the early boot before using the allocator.
    ///
    /// # Safety #
    ///
    /// The function must be called once during the early boot process with
    /// only one active CPU.
    pub unsafe fn init_all() {
        push_critical_region();

        // SAFETY: we are during the early boot process, there is only one CPU
        // active, and within a critical region: no interrupts or preemption.
        for (index, heap) in unsafe { HEAPS.iter_unchecked() }.enumerate() {
            unsafe {
                heap.borrow_mut().init(index as u8);
            }
        }

        pop_critical_region();
    }

    pub fn for_cpu(cpu_index: &CpuIndex) -> &RefCell<Self> {
        HEAPS.get(cpu_index)
    }

    pub fn pages_list(&mut self, size: usize) -> NonNull<RefCell<PageHeader>> {
        self.pages_list[Self::bucket_for_size(size) as usize]
    }

    pub fn small_alloc(&mut self, size: usize) -> NonNull<BlockHeader> {
        let bucket = (size + (SMALL_SIZE_BUCKET_INC - 1))
            >> SMALL_SIZE_BUCKET_INC_SHIFT;
        let mut page = unsafe { self.direct_pages[bucket].as_ref() }.borrow_mut();

        if let Some(block) = page.free_list {
            let block = unsafe { block.as_ref() };
            page.free_list = block.next;
            page.nr_block_used += 1;

            block.into()
        } else {
            self.generic_alloc(size);
            todo!()
        }
    }

    pub fn generic_alloc(&mut self, _size: usize) {
        // deferred free
        // heap delayed free
        // find or make a page from heap
        todo!()
    }

    pub fn acquire_free_page(&mut self) -> Option<NonNull<PageHeader>> {
        todo!()
    }

    pub fn bucket_for_size(size: usize) -> u8 {
        debug_assert!(size > 0);

        let wsize = align_up(size, size_of::<usize>()) / size_of::<usize>();

        if wsize == 1 {
            1
        } else if wsize <= 8 {
            align_up(wsize as u8, 2)
        } else {
            let wsize = wsize - 1;
            let bit_pos = first_bit_pos(wsize);

            ((((bit_pos as usize) << 2)
               | ((wsize >> (bit_pos - 2)) & 3)) - 3) as u8
        }
    }
}
