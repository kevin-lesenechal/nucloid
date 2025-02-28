/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::task::cpu::{CpuIndex, MAX_CPUS, NR_CPUS};
use core::sync::atomic::Ordering;

pub struct CpuLocal<T>([T; MAX_CPUS]);

// SAFETY: it is guaranteed that as long as we hold an instance of
// `CpuIndex`, we run on the associated CPU within a critical section,
// i.e. there is no possibility of interruption or preemption, so
// there is no risk of race-condition on accessing the array.
unsafe impl<T> Sync for CpuLocal<T> {}

impl<T> CpuLocal<T> {
    pub const fn new(items: [T; MAX_CPUS]) -> Self {
        Self(items)
    }

    /// Access the current CPU's value.
    pub fn get(&self, cpu_index: &CpuIndex) -> &T {
        // SAFETY: this function relies on the fact that as long as the
        // reference to `CpuIndex` is valid, the given CPU index is the current
        // executing CPU that won't change (through preemption or interruption)
        // during the reference's lifetime. The returned reference's lifetime is
        // therefor tied to the `CpuIndex`'s lifetime.
        &self.0[cpu_index.get()]
    }

    /// Iterate over all CPU-local variables (dangerous). Albeit dangerous, this
    /// function is useful to perform runtime initialization of CPU-local
    /// variables during the early boot process. *This is a niche function,
    /// you're probably not going to need it.*
    ///
    /// # Safety #
    ///
    /// This function may only be called during the early boot environment when
    /// only one CPU is running and with both preemption and interruptions
    /// disabled. Failing to do all that *will* lead to data-race conditions and
    /// invoke undefined behavior.
    pub unsafe fn iter_unchecked(&self) -> impl Iterator<Item = &T> {
        self.0.iter().take(NR_CPUS.load(Ordering::Relaxed))
    }
}

impl<T: Copy> CpuLocal<T> {
    pub const fn new_copy(item: T) -> Self {
        Self([item; MAX_CPUS])
    }
}
