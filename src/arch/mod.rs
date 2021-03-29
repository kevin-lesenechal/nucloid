/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use crate::arch::x86::export::*;
