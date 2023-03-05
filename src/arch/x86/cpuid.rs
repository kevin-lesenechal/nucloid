/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use x86::cpuid::CpuId;

static mut CPUID: Option<CpuId> = None;

pub unsafe fn init() {
    CPUID = Some(CpuId::new());
}

pub fn get() -> &'static CpuId {
    unsafe { CPUID.as_ref().expect("CPUID not initialized") }
}
