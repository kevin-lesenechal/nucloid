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

#[macro_use]
pub mod misc;

pub mod task;
pub mod ui;
mod backtrace;

fn main() -> ! {
    println!("Nucloid v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("\x1b<fg=cc7832>impl\x1b<!fg> PxFont {{");
    println!("    \x1b<fg=cc7832>pub fn\x1b<!fg> \x1b<fg=ffc66d>from_data\x1b<!fg>(data: &[\x1b<fg=cc7832>u8\x1b<!fg>]) -> Result<\x1b<fg=cc7832>Self,\x1b<!fg> PxFontError> {{");
    println!("        \x1b<fg=cc7832>let mut\x1b<!fg> reader = Cursor::\x1b<fg=ffc66d>new\x1b<!fg>(data)\x1b<fg=cc7832>;\x1b<!fg>");
    println!("        \x1b<fg=cc7832>let\x1b<!fg> header = FileHeader::\x1b<fg=ffc66d>read\x1b<!fg>(&\x1b<fg=cc7832>mut\x1b<!fg> reader)");
    println!("            .\x1b<fg=ffc66d>map_err\x1b<!fg>(|e| PxFontError::\x1b<fg=9876aa>InvalidHeader\x1b<!fg>(e))\x1b<fg=cc7832>?;\x1b<!fg>");
    println!("        \x1b<fg=cc7832>let\x1b<!fg> glyph_size = header.width \x1b<fg=cc7832>as\x1b<!fg> usize * header.height \x1b<fg=cc7832>as\x1b<!fg> usize\x1b<fg=cc7832>;\x1b<!fg>");
    println!("        \x1b<fg=cc7832>let mut\x1b<!fg> chars = HashMap::\x1b<fg=ffc66d>new\x1b<!fg>()\x1b<fg=cc7832>;\x1b<!fg>");
    println!();
    println!("Voix ambiguë d’un \x1b<fg=f00>cœur\x1b<!fg> qui, au \x1b<bg=2b2b2b>zéphyr\x1b<!bg>, préfère les jattes de \x1b<fg=0f0>kiwis\x1b<!fg>.");
    println!("В чащах юга жил бы цитрус? Да, но фальшивый экземпляр!");
    println!("Ξεσκεπάζω την ψυχοφθόρα σας βδελυγμία.");
    println!("Ça fera 1 035,00 €, ou £20.");
    println!("a\tbb\tccc\tdddd\teeeeee\teeeeeee\teeeeeeee\tf");
    println!("Hello \x1b<fg=ffc66d;bg=000000>WORLD\x1b<!fg;!bg>!");
    println!("Nucloid is powered by 🦀 \x1b<fg=f74c00>Rust\x1b<!fg>.");

    debug!("Test debug");
    info!("Un petite info");
    notice!("Avez-vous vu ça ?");
    warning!("Ceci est un warning !");
    error!("Oops, un erreur s'est produite...");
    critical!("Aïe ! C'est sérieux !");

    loop {
        arch::cpu::halt();
    }
}
