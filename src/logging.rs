/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::driver::vga::VgaScreen;

#[allow(dead_code)]
pub enum Severity {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl Severity {
    pub fn label(&self) -> &str {
        use Severity::*;

        match self {
            Debug       => "debug",
            Info        => "info",
            Notice      => "notice",
            Warning     => "warn",
            Error       => "error",
            Critical    => "critical",
            Alert       => "alert",
            Emergency   => "emerg",
        }
    }
}

pub trait Logger {
    fn log(&mut self, severity: Severity, message: &str);
}

#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => (
        $logger.log($crate::logging::Severity::Info)
    );
}

pub struct VgaLogger<V>
where V: VgaScreen + 'static
{
    vga: &'static mut V,
}

impl<V> VgaLogger<V>
where V: VgaScreen
{
    /*pub fn new(vga: &'static mut V) -> VgaLogger<V> {
        VgaLogger { vga }
    }*/
}

impl<V> Logger for VgaLogger<V>
where V: VgaScreen
{
    fn log(&mut self, severity: Severity, message: &str) {
        let attr = match severity {
            Severity::Debug     => 0x08,
            Severity::Info      => 0x07,
            Severity::Notice    => 0x0f,
            Severity::Warning   => 0x06,
            Severity::Error     => 0x04,
            Severity::Critical  => 0x0c,
            Severity::Alert     => 0x4f,
            Severity::Emergency => 0xce,
        };

        self.vga.set_attributes(attr);
        self.vga.put_str(severity.label());
        self.vga.set_attributes(0x07);
        self.vga.put_str(": ");
        self.vga.put_str(message);
        self.vga.put_char(b'\n');
    }
}
