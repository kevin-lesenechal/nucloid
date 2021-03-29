/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt;
use core::fmt::Formatter;

pub struct BinSize(pub u64);

impl fmt::Display for BinSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let size;
        let unit;

        if self.0 < 1024 {
            size = self.0 as f64;
            unit = "o";
        } else if self.0 < 1024 * 1024 {
            size = self.0 as f64 / 1024.0;
            unit = "Kio";
        } else if self.0 < 1024 * 1024 * 1024 {
            size = self.0 as f64 / 1024.0 / 1024.0;
            unit = "Mio";
        } else {
            size = self.0 as f64 / 1024.0 / 1024.0 / 1024.0;
            unit = "Gio";
        }

        write!(f, "{:.2} {}", size, unit)
    }
}

/// Returns the next integer multiple of `multiple` or `n` if already a
/// multiple of `multiple`.
pub const fn align_up(n: u64, multiple: u64) -> u64 {
    (n + (multiple - 1)) & !(multiple - 1)
}
