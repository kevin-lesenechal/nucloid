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
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::cpu::MachineState;
use crate::arch;
use crate::screen::VGA_SCREEN;
use crate::driver::vga::VgaScreen;

static PANIC_ENTERED: AtomicBool = AtomicBool::new(false);

pub fn panic_at_state(message: fmt::Arguments,
                      machine: Option<MachineState>) -> !
{
    // Make sure there is only one thread panicking; if another thread panicks,
    // we terminate it. This implies that the panic handler is non-reentrant
    // and, therefore, we must try our best not to trigger one in it.
    if PANIC_ENTERED.compare_exchange(false, true,
                                      Ordering::SeqCst, Ordering::SeqCst)
        .is_err() {
        arch::cpu::perm_halt();
    }

    // Safety: we don't want to deadlock on the VGA screen spinlock, so we
    // bypass it and hope for the best, we are about to die anyway.
    let vga = unsafe { &mut *VGA_SCREEN.bypass_lock() };

    // We do nothing if the VGA was not initialized, meaning we panicked very
    // early in the boot process and aren't able to print anything.
    if let Some(vga) = vga {
        print_panic(vga, message, machine);
    }

    arch::cpu::perm_halt();
}

#[allow(unused_must_use)]
fn print_panic(vga: &mut impl VgaScreen,
               message: fmt::Arguments,
               machine: Option<MachineState>)
{
    let (orig_x, mut anchor_y) = vga.cursor();

    if orig_x > 0 {
        write!(vga, "\n");
        anchor_y += 1;
    }

    vga.move_cursor(orig_x, anchor_y);
    vga.set_attributes(0x4f);
    write!(vga, "{:80}", "");
    vga.move_cursor(0, anchor_y);
    writeln!(vga, "PANIC! {}", message);

    if let Some(machine) = machine {
        vga.set_attributes(0x0f);
        machine.print(vga);
    }

    vga.set_attributes(0x1f);
    write!(vga, "{:80}", "STACK TRACE");

    vga.set_attributes(0x0f);
    writeln!(vga, "> arch_init()");
    vga.set_attributes(0x08);
    writeln!(vga, "    src/arch/x86/init.rs:124  <0xc0100600 + 0x2849>");
    vga.set_attributes(0x0f);
    writeln!(vga, "> _start()");
    vga.set_attributes(0x08);
    writeln!(vga, "    src/arch/x86/start32.S:194  <0xc0108bdf + 0x121>");

    vga.set_attributes(0x1f);
    write!(vga, "{:79}", "This was Nucloid v0.1.0, goodbye cruel world.");
}
