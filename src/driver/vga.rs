/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

pub trait VgaScreen: core::fmt::Write {
    fn put_char(&mut self, c: u8);

    fn put_str(&mut self, str: &str);

    fn println(&mut self, str: &str) {
        self.put_str(str);
        self.put_char(b'\n');
    }

    fn set_attributes(&mut self, attr: u8);

    fn move_cursor(&mut self, x: u8, y: u8);

    fn cursor(&self) -> (u8, u8);

    fn scroll_up(&mut self, lines: u8);

    fn clear(&mut self);
}
