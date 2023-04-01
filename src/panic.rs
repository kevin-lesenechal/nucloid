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
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(test))]
use core::panic::PanicInfo;

use crate::arch::cpu::MachineState;
use crate::{arch, print, println};
use crate::arch::logging::LOGGER_SERIAL;
use crate::backtrace::Backtrace;
use crate::driver::vga::VgaScreen;

static PANIC_ENTERED: AtomicBool = AtomicBool::new(false);

pub fn panic_at_state(
    message: fmt::Arguments,
    machine: Option<&MachineState>,
    skip_frames: usize,
) -> ! {
    // Make sure there is only one thread panicking; if another thread panics,
    // we terminate it. This implies that the panic handler is non-reentrant
    // and, therefore, we must try our best not to trigger one in it.
    if PANIC_ENTERED.compare_exchange(false, true,
                                      Ordering::SeqCst, Ordering::SeqCst)
        .is_err() {
        arch::cpu::perm_halt();
    }

    let logger = unsafe { LOGGER_SERIAL.as_mut() };

    // We do nothing if the screen was not initialized, meaning we panicked very
    // early in the boot process and aren't able to print anything.
    if let Some(logger) = logger {
        print_panic(logger, message, machine);
    }

    print_terminal(message, machine, skip_frames);

    arch::cpu::perm_halt();
}

#[allow(unused_must_use)]
fn print_panic_screen(
    vga: &mut impl VgaScreen,
    message: fmt::Arguments,
    machine: Option<&MachineState>,
) {
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

#[allow(unused_must_use)]
fn print_panic(
    w: &mut impl fmt::Write,
    message: fmt::Arguments,
    machine: Option<&MachineState>,
) {
    writeln!(w, "\x1b[31mPANIC! {}", message);
    if let Some(machine) = machine {
        writeln!(w, "{}", machine);
    }
}

fn print_terminal(
    message: fmt::Arguments,
    machine: Option<&MachineState>,
    skip_frames: usize,
) {
    println!("\x1b<nl;fg=f00;bg=ff0>KERNEL PANIC!\x1b<!bg> {message}");

    if let Some(machine) = machine {
        machine.print_term();

        for frame in Backtrace::from_machine_state(machine).skip(skip_frames) {
            if let Some(sym) = frame.symbol {
                println!("  > \x1b<fg=fff>{sym}\x1b<!fg>");
            } else {
                println!("  > ???");
            }
            print!("      ");
            if let Some((file, line)) = frame.file_line {
                print!("{file}:{line}    ");
            }
            if let Some(sym_off) = frame.sym_off {
                println!("<0x{:?} + {sym_off:#x}>", frame.pc);
            } else {
                println!("<0x{:?}>", frame.pc);
            }
        }
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let machine_state = MachineState::here();

    if let Some(msg) = panic_info.message() {
        panic_at_state(
            format_args!("Rust: {} ({})", msg, panic_info.location().unwrap()),
            Some(&machine_state),
            2,
        );
    } else {
        panic_at_state(
            format_args!("Rust panic with no message"),
            Some(&machine_state),
            2,
        );
    }
}
