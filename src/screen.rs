/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt;

use core::fmt::{Formatter, Write};
use crate::arch::logging::LOGGER_SERIAL;
use crate::mem::VAddr;

pub fn _print(args: fmt::Arguments) {
    let screen = unsafe { LOGGER_SERIAL.as_mut().unwrap() };
    screen.write_fmt(args).unwrap();
}

pub struct R<T>(pub T);

impl fmt::LowerHex for R<u32> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", &(self.0 >> 16))?;
        write!(f, "'")?;
        write!(f, "{:04x}", &(self.0 & 0xffff))
    }
}

impl fmt::LowerHex for R<u64> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", &(self.0 >> 32))?;
        write!(f, "'")?;
        write!(f, "{:08x}", &(self.0 & 0xffffffff))
    }
}

impl fmt::LowerHex for R<usize> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(target_pointer_width = "32")] {
            R(self.0 as u32).fmt(f)
        }
        #[cfg(target_pointer_width = "64")] {
            R(self.0 as u64).fmt(f)
        }
    }
}

impl fmt::LowerHex for R<VAddr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        R(self.0.0).fmt(f)
    }
}

struct NullScreen;

static mut NULL_SCREEN: NullScreen = NullScreen;

impl fmt::Write for NullScreen {
    fn write_str(&mut self, _s: &str) -> fmt::Result {
        Ok(())
    }
}
