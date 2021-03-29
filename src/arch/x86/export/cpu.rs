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

use crate::driver::vga::VgaScreen;
use core::fmt::Formatter;

pub struct MachineState {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,

    pub edi: u32,
    pub esi: u32,

    pub esp: u32,
    pub ebp: u32,

    pub eip: u32,
    pub eflags: u32,

    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
}

struct R<T>(T);

impl fmt::LowerHex for R<u32> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", &(self.0 >> 16))?;
        write!(f, "_")?;
        write!(f, "{:04x}", &(self.0 & 0xffff))
    }
}

impl MachineState {
    pub fn print(&self, vga: &mut impl VgaScreen) -> fmt::Result {
        writeln!(vga, "eax={:08x}   ebx={:08x}   ecx={:08x}   edx={:08x}",
                 R(self.eax), R(self.ebx), R(self.ecx), R(self.edx))?;
        writeln!(vga, "edi={:08x}   esi={:08x}   ebp={:08x}   esp={:08x}",
                 R(self.edi), R(self.esi), R(self.ebp), R(self.esp))?;
        writeln!(vga, "eip={:08x}   cs={:04x}   ds={:04x}   es={:04x}   fs={:04x}   gs={:04x}",
                 R(self.eip), self.cs, self.ds, self.es, self.fs, self.gs)?;
        writeln!(vga, "eflags={:08x}", R(self.eflags))
    }
}

pub fn halt() {
    unsafe { x86::halt(); }
}

pub fn perm_halt() -> ! {
    unsafe { x86::irq::disable() };
    loop {
        halt();
    }
}
