/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::cmp::Ordering;
use core::fmt::{Debug, Formatter, LowerHex};
use core::ops::{Add, AddAssign, Sub};
use crate::arch;
use crate::arch::cpu::MachineState;
use crate::panic::panic_at_state;

pub mod frame;
pub mod kalloc;
pub mod load;

pub use arch::mem::PAddr;

use crate::arch::mem::page_permissions;
use crate::screen::R;

pub static mut PHYS_MEM_SIZE: u64 = 0;
pub static mut LOWMEM_VA_END: VAddr = VAddr(0);

impl Add<u64> for PAddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct VAddr(pub usize);

impl VAddr {
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as _
    }

    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as _
    }
}

impl<T> From<*const T> for VAddr {
    fn from(ptr: *const T) -> Self {
        Self(ptr as usize)
    }
}

impl<T> From<*mut T> for VAddr {
    fn from(ptr: *mut T) -> Self {
        Self(ptr as usize)
    }
}

impl Add for VAddr {
    type Output = Self;

    fn add(self, rhs: VAddr) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<usize> for VAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for VAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for VAddr {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl PartialOrd for VAddr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Debug for VAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        R(*self).fmt(f)
    }
}

pub fn get_lowmem_va_end() -> VAddr {
    unsafe { LOWMEM_VA_END }
}

pub struct PagePermissions {
    pub accessible: bool,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

pub enum AccessAttempt {
    Read,
    Write,
    Execute,
}

pub fn handle_pagefault(fault_addr: VAddr,
                        access: AccessAttempt,
                        machine_state: &MachineState) {
    let op_str = match access {
        AccessAttempt::Read => "Invalid read",
        AccessAttempt::Write => "Invalid write",
        AccessAttempt::Execute => "Invalid execution",
    };

    let perms = page_permissions(fault_addr);
    let reason = if !perms.accessible {
        "page is not mapped"
    } else if matches!(access, AccessAttempt::Write) && !perms.writable {
        "page is read-only"
    } else if matches!(access, AccessAttempt::Execute) && !perms.executable {
        "page is non-executable"
    } else {
        "unknown error"
    };

    panic_at_state(
        format_args!("{} at {:?}: {}",
                     op_str, fault_addr, reason),
        Some(machine_state),
        0,
    );
}
