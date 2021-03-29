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
use crate::sync::Spinlock;

use crate::arch::x86::driver::vga::Vga;
use core::fmt::Write; // FIXME: indirection

pub static VGA_SCREEN: Spinlock<Option<Vga>> = Spinlock::new(None);

pub fn _print(args: fmt::Arguments) {
    let mut lock = VGA_SCREEN.lock();
    let vga = lock.as_mut().expect("VGA screen was not initialized");
    vga.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::screen::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}
