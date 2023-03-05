/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#![feature(panic_info_message)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]
#![feature(const_trait_impl)]
#![feature(inline_const)]
#![feature(const_for)]
#![feature(const_maybe_uninit_as_mut_ptr)]
#![feature(iter_advance_by)]

#![allow(unused_unsafe)]
#![allow(dead_code)]

#[cfg(not(test))]
extern crate alloc;

pub mod arch;
pub mod driver;
pub mod mem;
pub mod logging;
pub mod sync;
pub mod screen;
pub mod panic;
pub mod misc;
pub mod task;
pub mod ui;

#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use crate::panic::panic_at_state;

fn main() -> ! {
    println!("Hello, world!");
    println!();
    println!("impl PxFont {{");
    println!("    pub fn from_data(data: &[u8]) -> Result<Self, PxFontError> {{");
    println!("        let mut reader = Cursor::new(data);");
    println!("        let header = FileHeader::read(&mut reader)");
    println!("            .map_err(|e| PxFontError::InvalidHeader(e))?;");
    println!("        let glyph_size = header.width as usize * header.height as usize;");
    println!("        let mut chars = HashMap::new();");
    println!();
    println!("Voix ambiguë d’un \x1b<fg=f00>cœur\x1b<!fg> qui, au \x1b<bg=2b2b2b>zéphyr\x1b<!bg>, préfère les jattes de \x1b<fg=0f0>kiwis\x1b<!fg>.");
    println!("В чащах юга жил бы цитрус? Да, но фальшивый экземпляр!");
    println!("Ξεσκεπάζω την ψυχοφθόρα σας βδελυγμία.");
    println!("Ça fera 1 035,00 €, ou £20.");
    println!("a\tbb\tccc\tdddd\teeeeee\teeeeeee\teeeeeeee\tf");
    println!("Hello \x1b<fg=ffc66d;bg=000000>WORLD\x1b<!fg;!bg>!");
    println!("Nucloid is powered by 🦀 \x1b<fg=f74c00>Rust\x1b<!fg>. Jordan est un 🤡.");

    loop {
        arch::cpu::halt();
    }
}

#[cfg(not(test))]
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
