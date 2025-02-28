/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::driver::screen::{Color, FramebufferScreen};

pub struct VesaFramebuffer {
    mem: &'static mut [u32],
    width: usize,
    height: usize,
    pitch: usize,
    bpp: u8,
}

impl VesaFramebuffer {
    pub unsafe fn new(
        buffer: *mut u32,
        width: usize,
        height: usize,
        pitch: usize,
        bpp: u8,
    ) -> Self {
        let buff_size = pitch * height;

        assert_eq!(bpp, 32);

        VesaFramebuffer {
            mem: unsafe {
                core::slice::from_raw_parts_mut(buffer, buff_size >> 2)
            },
            width,
            height,
            pitch,
            bpp,
        }
    }
}

impl FramebufferScreen for VesaFramebuffer {
    fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn put(&mut self, x: usize, y: usize, color: Color) {
        let px =
            (color.r as u32) << 16 | (color.g as u32) << 8 | (color.b as u32);
        let index = (self.pitch >> 2) * y + x;
        self.mem[index] = px;
    }

    fn copy(&mut self, x: usize, y: usize, data: &[u32]) {
        let index = (self.pitch >> 2) * y + x;
        let target = &mut self.mem[index..(index + data.len())];
        target.copy_from_slice(data);
    }

    fn clear(&mut self) {
        todo!()
    }
}
