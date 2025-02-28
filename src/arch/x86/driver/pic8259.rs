/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::arch::x86::Ioport;
use x86::io::outb;

pub struct Pic8259 {
    master_port: Ioport,
    slave_port: Ioport,
}

impl Pic8259 {
    pub unsafe fn new(master_port: Ioport, slave_port: Ioport) -> Pic8259 {
        Pic8259 {
            master_port,
            slave_port,
        }
    }

    pub unsafe fn init(&mut self, master_vec_base: u8, slave_vec_base: u8) {
        unsafe {
            outb(self.master_port, 0b0001_0001);
            outb(self.slave_port, 0b0001_0001);
            outb(self.master_port + 1, master_vec_base);
            outb(self.slave_port + 1, slave_vec_base);
            outb(self.master_port + 1, 1 << 2); // Slave on IRQ 2
            outb(self.slave_port + 1, 1 << 1); // Slave ID 1
            outb(self.master_port + 1, 0b0000_0001);
            outb(self.slave_port + 1, 0b0000_0001);
            outb(self.master_port + 1, 0b0000_0000);
            outb(self.slave_port + 1, 0b0000_0000);
        }
    }

    pub fn ack_irq(&mut self, irq: u32) {
        if irq >= 8 {
            unsafe {
                outb(self.slave_port, 0x20);
            }
        }

        unsafe {
            outb(self.master_port, 0x20);
        }
    }
}
