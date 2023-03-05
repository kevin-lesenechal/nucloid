/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use x86::segmentation::{Descriptor as Descriptor32,
                        DescriptorBuilder, BuildDescriptor,
                        SegmentDescriptorBuilder, SegmentSelector, load_cs,
                        load_ss, load_ds, load_es, load_fs, load_gs,
                        GateDescriptorBuilder};
use x86::dtables::{DescriptorTablePointer, lgdt};
use x86::Ring::Ring0;
use x86::current::task::TaskStateSegment;
use x86::task::load_tr;

#[cfg(target_arch = "x86_64")]
use x86::bits64::segmentation::Descriptor64;

use crate::mem::{PAddr, VAddr};

#[cfg(target_arch = "x86_64")]
type DescriptorN = Descriptor64;

#[cfg(target_arch = "x86")]
type DescriptorN = Descriptor32;

#[cfg(target_arch = "x86_64")]
type UsizeT = u64;

#[cfg(target_arch = "x86")]
type UsizeT = u32;

#[derive(Default)]
#[repr(C, packed)]
struct Gdt {
    pub null: Descriptor32,
    pub kernel_cs: Descriptor32,
    pub kernel_ds: Descriptor32,
    pub user_cs32: Descriptor32,
    pub user_cs64: Descriptor32,
    pub user_ds: Descriptor32,
    pub tss: DescriptorN,
}

static mut BSP_GDT: Gdt = Gdt {
    null: Descriptor32::NULL,
    kernel_cs: Descriptor32::NULL,
    kernel_ds: Descriptor32::NULL,
    user_cs32: Descriptor32::NULL,
    user_cs64: Descriptor32::NULL,
    user_ds: Descriptor32::NULL,
    tss: DescriptorN::NULL,
};

pub const KERNEL_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, Ring0);

static mut BSP_TSS: TaskStateSegment = TaskStateSegment::new();

pub unsafe fn setup_table() {
    use x86::segmentation::CodeSegmentType::*;
    use x86::segmentation::DataSegmentType::*;
    use x86::Ring::*;

    let mut cs = DescriptorBuilder::code_descriptor(0, 0xfffff, ExecuteRead)
        .present()
        .dpl(Ring0)
        .limit_granularity_4kb();
    #[cfg(target_arch = "x86_64")] {
        cs = cs.l();
    }
    #[cfg(target_arch = "x86")] {
        cs = cs.db();
    }
    BSP_GDT.kernel_cs = cs.finish();

    BSP_GDT.kernel_ds =
        DescriptorBuilder::data_descriptor(0, 0xfffff, ReadWrite)
            .present()
            .dpl(Ring0)
            .limit_granularity_4kb()
            .db()
            .finish();
    BSP_GDT.user_cs32 =
        DescriptorBuilder::code_descriptor(0, 0xfffff, ExecuteRead)
            .present()
            .dpl(Ring3)
            .limit_granularity_4kb()
            .db()
            .finish();
    #[cfg(target_arch = "x86_64")] {
        BSP_GDT.user_cs64 =
            DescriptorBuilder::code_descriptor(0, 0xfffff, ExecuteRead)
                .present()
                .dpl(Ring3)
                .limit_granularity_4kb()
                .l()
                .finish();
    }
    BSP_GDT.user_ds =
        DescriptorBuilder::data_descriptor(0, 0xfffff, ReadWrite)
            .present()
            .dpl(Ring3)
            .limit_granularity_4kb()
            .db()
            .finish();

    BSP_GDT.tss =
        <DescriptorBuilder as GateDescriptorBuilder<UsizeT>>::tss_descriptor(
            PAddr::from_lowmem_vaddr(VAddr(&BSP_TSS as *const _ as _)).unwrap().0 as _,
            core::mem::size_of_val(&BSP_TSS) as _,
            true
        ).present()
        .finish();

    let ptr = DescriptorTablePointer::new(&BSP_GDT);
    lgdt(&ptr);
}

pub unsafe fn load_kernel_selectors() {
    use x86::Ring::*;

    load_cs(SegmentSelector::new(1, Ring0));
    load_ss(SegmentSelector::new(2, Ring0));
    load_ds(SegmentSelector::new(2, Ring0));
    load_es(SegmentSelector::new(2, Ring0));
    load_fs(SegmentSelector::new(2, Ring0));
    load_gs(SegmentSelector::new(2, Ring0));
    load_tr(SegmentSelector::new(6, Ring0));
}
