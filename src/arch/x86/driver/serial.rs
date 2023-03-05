/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use x86::io::{outb, inb};
use core::fmt;
use core::fmt::Write;

use crate::logging::{Logger, Severity};

pub const COM1_IOPORT: u16 = 0x03f8;
pub const COM2_IOPORT: u16 = 0x02f8;
pub const COM3_IOPORT: u16 = 0x03e8;
pub const COM4_IOPORT: u16 = 0x02e8;

const REG_DATA: u16         = 0; // DLAB = 0
const REG_DIVISOR_LSB: u16  = 0; // DLAB = 1
const REG_IRQ_ENABLE: u16   = 1; // DLAB = 0
const REG_DIVISOR_MSB: u16  = 1; // DLAB = 1
const REG_IRQ_ID: u16       = 2;
const REG_LINE_CTRL: u16    = 3;
const REG_MODEM_CTRL: u16   = 4;
const REG_LINE_STATUS: u16  = 5;
const REG_MODEM_STATUS: u16 = 6;
const REG_SCRATCH: u16      = 7;

pub struct SerialDevice {
    ioport_base: u16,
    baud_rate: u32,
    parity: ParityMode,
    bits: u8,
    stop_bits: StopBits,
}

pub enum ParityMode {
    None,
    Odd,
    Even,
    Mark,
    Space,
}

pub enum StopBits {
    One,
    Two,
}

impl SerialDevice {
    pub unsafe fn new(ioport_base: u16,
                      baud_rate: u32,
                      parity: ParityMode,
                      bits: u8,
                      stop_bits: StopBits) -> Result<Self, &'static str> {
        let mut dev = Self {
            ioport_base,
            baud_rate,
            parity,
            bits,
            stop_bits,
        };

        dev.init()?;

        Ok(dev)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        let divisor: u16 = match self.baud_rate {
            115_200 => 1,
            57_600  => 2,
            38_400  => 3,
            19_200  => 6,
            9600    => 12,
            4800    => 24,
            2400    => 48,
            1200    => 96,
            600     => 192,
            300     => 384,
            220     => 524,
            110     => 1047,
            50      => 2304,
            _ => return Err("Unsupported baud rate, no divisor available"),
        };

        let parity_bits = match &self.parity {
            ParityMode::None    => 0b000,
            ParityMode::Odd     => 0b001,
            ParityMode::Even    => 0b011,
            ParityMode::Mark    => 0b101,
            ParityMode::Space   => 0b111,
        };
        let stop_bits = match &self.stop_bits {
            StopBits::One => 0,
            StopBits::Two => 1,
        };
        let bits = match self.bits {
            5 => 0b00,
            6 => 0b01,
            7 => 0b10,
            8 => 0b11,
            _ => return Err("Unsupported number of data bits"),
        };
        let line_ctrl: u8 = (parity_bits << 3) | (stop_bits << 2) | (bits << 0);

        unsafe {
            outb(self.ioport_base + REG_LINE_CTRL, 1 << 7); // DLAB = 1
            outb(self.ioport_base + REG_DIVISOR_MSB, (divisor >> 8) as u8);
            outb(self.ioport_base + REG_DIVISOR_LSB, (divisor & 0xff) as u8);
            outb(self.ioport_base + REG_LINE_CTRL, line_ctrl); // DLAB = 0
            outb(self.ioport_base + REG_IRQ_ENABLE, 0x00);
        }

        Ok(())
    }

    pub fn may_read(&self) -> bool {
        (unsafe { inb(self.ioport_base + REG_LINE_STATUS) } & (1 << 0)) > 0
    }

    pub fn read_blocking(&self) -> u8 {
        while !self.may_read() {}

        unsafe {
            inb(self.ioport_base + REG_DATA)
        }
    }

    pub fn may_write(&self) -> bool {
        (unsafe { inb(self.ioport_base + REG_LINE_STATUS) } & (1 << 5)) > 0
    }

    pub fn write_byte(&mut self, byte: u8) {
        while !self.may_write() {}

        unsafe {
            outb(self.ioport_base + REG_DATA, byte);
        }
    }
}

impl fmt::Write for SerialDevice {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes().iter() {
            self.write_byte(byte);
        }

        Ok(())
    }
}

impl Logger for SerialDevice {
    fn log(&mut self, severity: Severity, args: fmt::Arguments) {
        let (color, severity_str) = match severity {
            Severity::Debug => ("\x1b[90m", "debug"),
            Severity::Info => ("\x1b[37m", "info"),
            Severity::Notice => ("\x1b[97m", "notice"),
            Severity::Warning => ("\x1b[93m", "warning"),
            Severity::Error => ("\x1b[31m", "error"),
            Severity::Critical => ("\x1b[1;31m", "critic."),
            Severity::Alert => ("\x1b[1;97;41m", "ALERT"),
            Severity::Emergency => ("\x1b[1;93;41m", "EMERG."),
        };

        write!(self, "{}{:>7}: ", color, severity_str).unwrap();
        self.write_fmt(args).unwrap();
        write!(self, "\x1b[0m\n").unwrap();
    }
}
