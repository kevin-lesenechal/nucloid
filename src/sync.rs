/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;

use crate::arch::sync::{push_critical_region, pop_critical_region};

pub struct Spinlock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> SpinlockGuard<T> {
        push_critical_region();
        while self.lock.compare_exchange_weak(false, true,
                                              Ordering::Acquire,
                                              Ordering::Relaxed).is_err() {
            pop_critical_region();
            while self.is_locked() {
                core::hint::spin_loop();
            }
            push_critical_region();
        }

        // Safety: The spinlock guarantees exclusive access to the resource
        // wrapped inside it, we just acquired the lock, we are the only owner
        // of the resource so we can create a mutable reference to it.
        let data = unsafe { &mut *self.data.get() };

        SpinlockGuard {
            lock: &self.lock,
            data,
        }
    }

    /// Checks whether the lock is held right now, without any lock or
    /// synchronization.
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub unsafe fn bypass_lock(&self) -> *mut T {
        self.data.get()
    }
}

pub struct SpinlockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        pop_critical_region();
    }
}
