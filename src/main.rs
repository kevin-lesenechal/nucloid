/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

#![no_std]
#![no_main]

#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(const_mut_refs)]
#![feature(panic_info_message)]

#![allow(unused_unsafe)]
#![allow(dead_code)]

mod arch;
mod driver;
mod mem;
mod logging;
mod sync;
mod screen;
mod panic;
mod misc;
mod task;

use core::panic::PanicInfo;

use crate::panic::panic_at_state;

fn main() -> ! {
    loop {
        arch::cpu::halt();
    }
}

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    if let Some(msg) = panic_info.message() {
        panic_at_state(
            format_args!("Rust: {} ({})", msg, panic_info.location().unwrap()),
            None
        );
    } else {
        panic_at_state(
            format_args!("Rust panic with no message"),
            None
        );
    }
}
