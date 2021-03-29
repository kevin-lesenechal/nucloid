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
use core::fmt::{Formatter, Display};

#[cfg(target_arch = "x86")]
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
    pub ss: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
}

#[cfg(target_arch = "x86_64")]
pub struct MachineState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,

    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    pub rdi: u64,
    pub rsi: u64,

    pub rsp: u64,
    pub rbp: u64,

    pub rip: u64,
    pub rflags: u64,

    pub cs: u16,
    pub ss: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
}

#[cfg(target_arch = "x86_64")]
impl MachineState {
    pub fn print(&self, vga: &mut impl VgaScreen) -> fmt::Result {
        writeln!(vga, "rax{:016x} rbx{:016x} rcx{:016x} rdx{:016x}",
                 self.rax, self.rbx, self.rcx, self.rdx)?;
        writeln!(vga, "rdi{:016x} rsi{:016x} rbp{:016x} rsp{:016x}",
                 self.rdi, self.rsi, self.rbp, self.rsp)?;
        writeln!(vga, "r8 {:016x} r9 {:016x} r10{:016x} r11{:016x}",
                 self.r8, self.r9, self.r10, self.r11)?;
        writeln!(vga, "r12{:016x} r13{:016x} r14{:016x} r15{:016x}",
                 self.r12, self.r13, self.r14, self.r15)?;
        writeln!(vga, "rip={:016x}   cs={:04x}   ss={:04x}   ds={:04x}   es={:04x}   fs={:04x}   gs={:04x}",
                 self.rip, self.cs, self.ss, self.ds, self.es, self.fs, self.gs)
    }
}

#[cfg(target_arch = "x86_64")]
impl Display for MachineState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use crate::screen::R;

        writeln!(f, "rax={:x}  rbx={:x}  rcx={:x}  rdx={:x}",
                 R(self.rax), R(self.rbx), R(self.rcx), R(self.rdx))?;
        writeln!(f, "rdi={:x}  rsi={:x}  rbp={:x}  rsp={:x}",
                 R(self.rdi), R(self.rsi), R(self.rbp), R(self.rsp))?;
        writeln!(f, " r8={:x}   r9={:x}  r10={:x}  r11={:x}",
                 R(self.r8), R(self.r9), R(self.r10), R(self.r11))?;
        writeln!(f, "r12={:x}  r13={:x}  r14={:x}  r15={:x}",
                 R(self.r12), R(self.r13), R(self.r14), R(self.r15))?;
        writeln!(f, "rip={:016x}   cs={:04x}   ss={:04x}   ds={:04x}   es={:04x}   fs={:04x}   gs={:04x}",
                 self.rip, self.cs, self.ss, self.ds, self.es, self.fs, self.gs)
    }
}

#[cfg(target_arch = "x86")]
impl Display for MachineState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use crate::screen::R;

        writeln!(f, "eax={:x}  ebx={:x}  ecx={:x}  edx={:x}",
                 R(self.eax), R(self.ebx), R(self.ecx), R(self.edx))?;
        writeln!(f, "edi={:x}  esi={:x}  ebp={:x}  esp={:x}",
                 R(self.edi), R(self.esi), R(self.ebp), R(self.esp))?;
        writeln!(f, "eip={:x}   cs={:04x}   ss={:04x}   ds={:04x}   es={:04x}   fs={:04x}   gs={:04x}",
                 R(self.eip), self.cs, self.ss, self.ds, self.es, self.fs, self.gs)
    }
}

#[cfg(target_arch = "x86")]
impl MachineState {
    pub fn print(&self, vga: &mut impl VgaScreen) -> fmt::Result {
        use crate::screen::R;

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
