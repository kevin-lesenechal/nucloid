/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
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
mod libc;

use crate::driver::screen::FramebufferScreen;
use crate::ui::term::Terminal;

#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use crate::panic::panic_at_state;

fn main(fb: impl FramebufferScreen) -> ! {
    let mut t = Terminal::create(fb);
    t.write("impl PxFont {\n");
    t.write("    pub fn from_data(data: &[u8]) -> Result<Self, PxFontError> {\n");
    t.write("        let mut reader = Cursor::new(data);\n");
    t.write("        let header = FileHeader::read(&mut reader)\n");
    t.write("            .map_err(|e| PxFontError::InvalidHeader(e))?;\n");
    t.write("        let glyph_size = header.width as usize * header.height as usize;\n");
    t.write("        let mut chars = HashMap::new();\n");
    t.write("\n");
    t.write("Voix ambiguë d’un \x1b{fg=f00}cœur\x1b{fg=!} qui, au zéphyr, préfère les jattes de \x1b{fg=0f0}kiwis\x1b{fg=!}.\n");
    t.write("В чащах юга жил бы цитрус? Да, но фальшивый экземпляр!\n");
    t.write("Ξεσκεπάζω την ψυχοφθόρα σας βδελυγμία.\n");
    t.write("Ça fera 1 035,00 €, ou £20.\n");
    t.write("a\tbb\tccc\tdddd\teeeeee\teeeeeee\teeeeeeee\tf\n");
    t.write("Hello \x1b{fg=ffc66d;bg=000000}WORLD\x1b{fg=!}!");

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
