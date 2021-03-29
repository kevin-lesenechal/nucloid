/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::sync::atomic::{AtomicU32, Ordering};

// FIXME: implement per SMP processor
static CRITICAL_REGION_DEPTH: AtomicU32 = AtomicU32::new(0);

pub fn push_critical_region() {
    let prev = CRITICAL_REGION_DEPTH.fetch_add(1, Ordering::SeqCst);

    if prev == 0 {
        unsafe { x86::irq::disable() };
    }
}

pub fn pop_critical_region() {
    let prev = CRITICAL_REGION_DEPTH.fetch_sub(1, Ordering::SeqCst);

    if prev == 1 {
        unsafe { x86::irq::enable() };
    }
}
