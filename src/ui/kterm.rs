/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt::{Arguments, Write};

use crate::arch::VesaFramebuffer;
use crate::logging::{Logger, Severity};
use crate::sync::Spinlock;
use crate::ui::term::Terminal;

pub static KERNEL_TERMINAL: Spinlock<Option<Terminal<VesaFramebuffer>>> =
    Spinlock::new(None);

pub struct TerminalLogger {
    serial: &'static mut (dyn Logger + Send),
}

impl TerminalLogger {
    pub fn new(serial: &'static mut (dyn Logger + Send)) -> Self {
        Self { serial }
    }
}

impl Logger for TerminalLogger {
    fn log(&mut self, severity: Severity, args: Arguments) {
        self.serial.log(severity, args.clone());

        let (color, severity_str) = match severity {
            Severity::Debug => ("\x1b<fg=686868>", "debug"),
            Severity::Info => ("\x1b<fg=b2b2b2>", "info"),
            Severity::Notice => ("\x1b<fg=ffffff>", "notice"),
            Severity::Warning => ("\x1b<fg=ffff54>", "warning"),
            Severity::Error => ("\x1b<fg=b21818>", "error"),
            Severity::Critical => ("\x1b<fg=ff0000>", "critic."),
            Severity::Alert => ("\x1b<fg=ff0000>", "ALERT"),
            Severity::Emergency => ("\x1b<fg=ff0000>", "EMERG."),
        };
        let mut kterm = KERNEL_TERMINAL.lock();
        if let Some(ref mut kterm) = *kterm {
            write!(kterm, "{}{:>7}: ", color, severity_str).unwrap();
            kterm.write_fmt(args).unwrap();
            write!(kterm, "\x1b<!fg>\n").unwrap();
        }
    }
}

pub fn _print(args: Arguments) {
    let mut kterm = KERNEL_TERMINAL.lock();
    if let Some(ref mut kterm) = *kterm {
        let _ = kterm.write_fmt(args);
    }
}

pub fn _fprint(args: Arguments) {
    let mut kterm = KERNEL_TERMINAL.lock();
    if let Some(ref mut kterm) = *kterm {
        let _ = kterm.write_fmt(args);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::ui::kterm::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! fprint {
    ($($arg:tt)*) => {
        $crate::ui::kterm::_fprint(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! fprintln {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        $crate::fprint!("{}\n", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! dbg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `eprintln!`
    // will be malformed.
    () => {
        $crate::debug!("[{}:{}]", core::file!(), core::line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::debug!("[{}:{}] {} = {:#?}",
                    core::file!(), core::line!(), core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
