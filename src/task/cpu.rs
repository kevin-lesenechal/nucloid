/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::sync::atomic::AtomicUsize;
use crate::arch::sync::{push_critical_region, pop_critical_region};

pub const MAX_CPUS: usize = 32;
pub static NR_CPUS: AtomicUsize = AtomicUsize::new(0);

pub struct CpuIndex(usize);

impl CpuIndex {
    /// Warning! Avoid copying the return value, but rather use it directly.
    /// In fact, once the `CpuIndex` is dropped, there is no more guarantee that
    /// the returned CPU index will be the current executing CPU's index: the
    /// current task could be preempted or interrupted and rescheduled to
    /// another CPU. Always ensure that the `&self` reference outlives the
    /// numerical value.
    pub fn get(&self) -> usize {
        self.0
    }
}

impl Drop for CpuIndex {
    fn drop(&mut self) {
        pop_critical_region();
    }
}

pub fn current_cpu_index() -> CpuIndex {
    push_critical_region();

    let curr_cpu = 0; // TODO

    CpuIndex(curr_cpu)
}
