/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::arch;

pub mod frame;

pub use arch::mem::PAddr;

pub static mut PHYS_MEM_SIZE: u64 = 0;
pub static mut VA_SIZE: usize = 0;

pub fn get_va_size() -> usize {
    unsafe { VA_SIZE }
}

pub struct PagePermissions {
    pub accessible: bool,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}
