/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::arch::x86::cpuid;

pub fn is_supported() -> bool {
    if let Some(features) = cpuid::get().get_feature_info() {
        features.has_apic()
    } else {
        false
    }
}

mod register {
    pub const LOCAL_APIC_ID: usize = 0x20;
    pub const LOCAL_APIC_VERSION: usize = 0x30;
    pub const EOI: usize = 0xb0;
}

pub struct Apic {
    regs: *mut u32,
}

impl Apic {
    pub unsafe fn new(registers: *mut u32) -> Apic {
        Apic { regs: registers }
    }

    pub fn eoi(&self) {
        self.write(register::EOI, 0);
    }

    fn write(&self, reg: usize, value: u32) {
        let index = reg >> 2;
        assert!(index < 252);

        unsafe {
            core::ptr::write_volatile(self.regs.add(index), value);
        }
    }
}
