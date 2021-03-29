/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use x86::segmentation::{Descriptor, DescriptorBuilder, BuildDescriptor,
                        SegmentDescriptorBuilder, SegmentSelector,
                        load_cs, load_ss, load_ds, load_es, load_fs, load_gs};
use x86::dtables::{DescriptorTablePointer, lgdt};
use x86::Ring::Ring0;

static mut GDT: [Descriptor; 5] = [
    Descriptor::NULL,
    Descriptor::NULL, // Kernel CS
    Descriptor::NULL, // Kernel SS/DS/ES
    Descriptor::NULL, // User CS
    Descriptor::NULL, // User SS/DS/ES
];

pub const KERNEL_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(1, Ring0);

pub unsafe fn setup_table() {
    use x86::segmentation::CodeSegmentType::*;
    use x86::segmentation::DataSegmentType::*;
    use x86::Ring::*;

    GDT[1] =
        DescriptorBuilder::code_descriptor(0, 0xfffff, ExecuteRead)
            .present()
            .dpl(Ring0)
            .limit_granularity_4kb()
            .db()
            .finish();
    GDT[2] =
        DescriptorBuilder::data_descriptor(0, 0xfffff, ReadWrite)
            .present()
            .dpl(Ring0)
            .limit_granularity_4kb()
            .db()
            .finish();
    GDT[3] =
        DescriptorBuilder::code_descriptor(0, 0xfffff, ExecuteRead)
            .present()
            .dpl(Ring3)
            .limit_granularity_4kb()
            .db()
            .finish();
    GDT[4] =
        DescriptorBuilder::data_descriptor(0, 0xfffff, ReadWrite)
            .present()
            .dpl(Ring3)
            .limit_granularity_4kb()
            .db()
            .finish();

    let ptr = DescriptorTablePointer::new(&GDT);
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
}
