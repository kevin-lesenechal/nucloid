/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::mem::transmute;
use core::str::FromStr;

#[derive(Debug, Copy, Clone, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C, align(4))]
pub struct ColorA {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color {
    pub fn from_bgra_u32(bgra: u32) -> Self {
        Self {
            r: ((bgra & 0x00ff0000) >> 16) as u8,
            g: ((bgra & 0x0000ff00) >> 8) as u8,
            b: (bgra & 0x000000ff) as u8,
        }
    }

    pub fn blend(fg: Color, alpha: u8, bg: Color) -> Color {
        Color {
            r: Self::blend_channel(fg.r, alpha, bg.r),
            g: Self::blend_channel(fg.g, alpha, bg.g),
            b: Self::blend_channel(fg.b, alpha, bg.b),
        }
    }

    pub fn with_alpha(self, a: u8) -> ColorA {
        ColorA {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    #[inline]
    pub fn as_bgra(self) -> [u8; 4] {
        [self.b, self.g, self.r, 255]
    }

    #[inline]
    fn blend_channel(fg: u8, alpha: u8, bg: u8) -> u8 {
        let fg = (alpha as u16 * fg as u16 / 255) as u8;
        let bg = ((255 - alpha) as u16 * bg as u16 / 255) as u8;

        fg + bg
    }
}

impl ColorA {
    #[inline]
    pub fn from_bgra_u32(bgra: u32) -> Self {
        unsafe { transmute(bgra) }
    }

    #[inline]
    pub fn blend(self, other: ColorA) -> ColorA {
        if self.a == 0 {
            other
        } else if self.a == 255 {
            self
        } else {
            ColorA {
                r: Color::blend_channel(self.r, self.a, other.r),
                g: Color::blend_channel(self.g, self.a, other.g),
                b: Color::blend_channel(self.b, self.a, other.b),
                a: 255,
            }
        }
    }

    #[inline]
    pub fn as_bgra(self) -> [u8; 4] {
        unsafe { transmute(self) }
    }

    #[inline]
    pub fn as_bgra_u32(self) -> u32 {
        unsafe { transmute(self) }
    }

    #[inline]
    pub fn as_rgb(self) -> Color {
        Color { r: self.r, g: self.g, b: self.b }
    }
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 3 {
            let rgb = u16::from_str_radix(s, 16).map_err(|_| ())?;
            Ok(Color {
                r: (((rgb & 0x0f00) >> 8) << 4) as u8,
                g: (((rgb & 0x00f0) >> 4) << 4) as u8,
                b: (((rgb & 0x000f) >> 0) << 4) as u8,
            })
        } else if s.len() == 6 {
            let rgb = u32::from_str_radix(s, 16).map_err(|_| ())?;
            Ok(Color {
                r: ((rgb & 0x00ff_0000) >> 16) as u8,
                g: ((rgb & 0x0000_ff00) >> 8) as u8,
                b: ((rgb & 0x0000_00ff) >> 0) as u8,
            })
        } else {
            Err(())
        }
    }
}

pub trait FramebufferScreen {
    fn dimensions(&self) -> (usize, usize);

    fn put(&mut self, x: usize, y: usize, color: Color);

    fn copy(&mut self, x: usize, y: usize, data: &[u32]);

    fn clear(&mut self);
}

pub struct CharAttrs {
    pub color: Color,
}

pub trait TextScreen {
    fn put(&mut self, x: usize, y: usize, c: char, attrs: CharAttrs);

    fn scroll_up(&mut self, lines: u8);

    fn clear(&mut self);
}

pub struct FramebufferTextScreen<F: FramebufferScreen> {
    fb: F,
}

impl<F: FramebufferScreen> TextScreen for FramebufferTextScreen<F> {
    fn put(&mut self, _x: usize, _y: usize, _c: char, _attrs: CharAttrs) {
        todo!()
    }

    fn scroll_up(&mut self, _lines: u8) {
        todo!()
    }

    fn clear(&mut self) {
        todo!()
    }
}
