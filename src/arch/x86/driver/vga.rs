/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::driver::vga::VgaScreen;

use core::fmt;
use core::slice;

pub struct Vga<'a> {
    mem: &'a mut [u8],
    width: u8,
    height: u8,
    curs_x: u8,
    curs_y: u8,
    attr: u8,
}

impl<'a> Vga<'a> {
    pub unsafe fn new(
        addr: *mut u8,
        size: usize,
        width: u8,
        height: u8,
    ) -> Self {
        assert_eq!(size, width as usize * height as usize * 2);
        Self {
            mem: unsafe { slice::from_raw_parts_mut(addr, size) },
            width,
            height,
            curs_x: 0,
            curs_y: 0,
            attr: 0x07,
        }
    }

    fn cursor_index(&self) -> usize {
        (self.curs_y as usize * self.width as usize * 2)
            + (self.curs_x as usize * 2)
    }
}

impl<'a> VgaScreen for Vga<'a> {
    fn put_char(&mut self, c: u8) {
        match c {
            b'\n' => {
                self.curs_x = 0;
                self.curs_y += 1;
            }
            b'\t' => {
                self.curs_x += 8 - (self.curs_x % 8);
            }
            b'\r' => {
                self.curs_x = 0;
            }
            _ => {
                let index = self.cursor_index();
                self.mem[index] = c;
                self.mem[index + 1] = self.attr;
                self.curs_x += 1;
            }
        }

        if self.curs_x >= self.width {
            self.curs_x = 0;
            self.curs_y += 1;
        }

        if self.curs_y >= self.height {
            self.scroll_up(self.curs_y - self.height + 1);
        }
    }

    fn put_str(&mut self, str: &str) {
        for c in str.chars() {
            if c.is_ascii() {
                self.put_char(c as u8);
            } else {
                self.put_char(b'?');
            }
        }
    }

    fn set_attributes(&mut self, attr: u8) {
        self.attr = attr;
    }

    fn move_cursor(&mut self, x: u8, y: u8) {
        assert!(x < self.width && y < self.height);
        self.curs_x = x;
        self.curs_y = y;
    }

    fn cursor(&self) -> (u8, u8) {
        (self.curs_x, self.curs_y)
    }

    fn scroll_up(&mut self, lines: u8) {
        let start = lines as usize * self.width as usize * 2;
        let len = (self.height - lines) as usize * self.width as usize * 2;

        self.mem.copy_within(start..(start + len), 0);
        self.mem[len..].fill(0);
        self.curs_y -= lines;
    }

    fn clear(&mut self) {
        self.mem.fill(0);
        self.curs_x = 0;
        self.curs_y = 0;
    }
}

impl<'a> fmt::Write for Vga<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_str(s);
        Ok(())
    }
}
