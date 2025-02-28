/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::mem::transmute;
use core::str::FromStr;

use crate::driver::screen::{Color, ColorA, FramebufferScreen};
use crate::ui::pxfont::PxFont;

const DEFAULT_FG_COLOR: Color = Color {
    r: 169,
    g: 183,
    b: 198,
};

pub struct Terminal<Fb> {
    background: &'static [u8],
    font: PxFont,
    fb: Fb,
    width_px: usize,
    height_px: usize,
    columns: usize,
    rows: usize,
    cursor_x: usize,
    cursor_y: usize,
    curr_style: GlyphStyle,
    cells: VecDeque<TermCell>,
    back_buffer: VecDeque<ColorA>,
}

#[derive(Copy, Clone)]
struct GlyphStyle {
    fg_color: Color,
    bg_color: Option<Color>,
}

#[derive(Copy, Clone)]
struct TermCell {
    c: char,
    style: GlyphStyle,
}

impl<Fb: FramebufferScreen> Terminal<Fb> {
    pub fn create(fb: Fb) -> Self {
        let (width_px, height_px) = (fb.dimensions().0, fb.dimensions().1);
        let font = PxFont::from_data(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/media/iosevka.pxfont"
        )))
        .unwrap();
        let columns = width_px / font.glyph_width() as usize;
        let rows = height_px / font.glyph_height() as usize;

        let mut term = Self {
            background: include_bytes_align_as!(
                u32,
                concat!(env!("CARGO_MANIFEST_DIR"), "/media/wallpaper.data")
            ),
            font,
            fb,
            width_px,
            height_px,
            columns,
            rows,
            cursor_x: 0,
            cursor_y: 0,
            curr_style: Default::default(),
            cells: VecDeque::new(),
            back_buffer: VecDeque::with_capacity(width_px * height_px),
        };
        term.clear();

        term
    }

    pub fn clear(&mut self) {
        self.clear_visual();

        self.cells = vec![Default::default(); self.rows * self.columns].into();
        self.back_buffer =
            vec![Default::default(); self.width_px * self.height_px].into();
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn clear_visual(&mut self) {
        let dst_row_len = self.fb.dimensions().0;
        let mut src_row = unsafe { self.background.align_to::<u32>().1 };

        for y in 0..self.fb.dimensions().1 {
            self.fb.copy(0, y, unsafe {
                (&src_row[..dst_row_len]).align_to::<u32>().1
            });
            src_row = &src_row[1920..];
        }
    }

    pub fn write(&mut self, s: &str) {
        let mut it = s.char_indices();

        while let Some((_i, c)) = it.next() {
            /*if c == '\x1b' {
                let _ = it.advance_by(self.handle_escape(&s[(i + 1)..]));
            }*/
            self.putc(c);
        }
    }

    pub fn putc(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.advance_y();
            }
            '\t' => self.advance_x(8 - (self.cursor_x & 0b111)),
            ' ' | '\u{a0}' | '\u{202f}' => self.advance_x(1),
            //'\x1b' => return, // TODO: remove
            '\x00'..='\x1f' => {
                self.write_char((0x2400 + c as u32).try_into().unwrap())
            }
            '\x7f' => self.write_char('\u{2421}'),
            c => self.write_char(c),
        }
    }

    fn write_char(&mut self, c: char) {
        let glyph_size = self.glyph_size(c);
        if self.cursor_x + glyph_size >= self.columns {
            self.cursor_x = 0;
            self.advance_y();
        }
        self.render_glyph(c, self.cursor_x, self.cursor_y, self.curr_style);
        *self.cell_at(self.cursor_x, self.cursor_y) = TermCell {
            c,
            style: self.curr_style,
        };
        for i in (self.cursor_x + 1)..(self.cursor_x + glyph_size) {
            *self.cell_at(i, self.cursor_y) = TermCell {
                c: '\0',
                style: self.curr_style,
            };
        }
        self.advance_x(glyph_size);
    }

    pub fn scroll_up(&mut self, mut nr_lines: usize) {
        if nr_lines > self.rows {
            nr_lines = self.rows;
        }

        for _ in 0..(nr_lines * self.columns) {
            self.cells.pop_front();
        }
        for _ in 0..(nr_lines * self.columns) {
            self.cells.push_back(TermCell::default());
        }

        let mid = nr_lines * self.font.glyph_height() as usize * self.width_px;
        let full_size = self.width_px * self.height_px;
        self.back_buffer.rotate_left(mid);
        for i in (full_size - mid)..full_size {
            self.back_buffer[i] = ColorA::default();
        }

        self.rerender();

        self.cursor_y = self.cursor_y.saturating_sub(nr_lines);
    }

    fn rerender(&mut self) {
        let mut back = Vec::with_capacity(self.width_px * self.height_px);
        let (_, bg, _) = unsafe { self.background.align_to() };

        for (i, &px) in self.back_buffer.iter().enumerate() {
            let bg_color =
                ColorA::from_bgra_u32(unsafe { *bg.get_unchecked(i) });
            back.push(px.blend(bg_color).as_bgra_u32());
        }

        self.fb.copy(0, 0, &back);
    }

    fn advance_x(&mut self, by: usize) {
        self.cursor_x += by;
        if self.cursor_x >= self.columns {
            self.cursor_x = 0;
            self.advance_y();
        }
    }

    fn advance_y(&mut self) {
        if self.cursor_y == self.rows - 1 {
            self.scroll_up(1);
        }

        self.cursor_y += 1;
    }

    fn glyph_size(&self, c: char) -> usize {
        self.font
            .get_glyph(c)
            .unwrap_or(self.font.replacement_glyph())
            .nr_columns()
    }

    fn render_glyph(&mut self, c: char, x: usize, y: usize, style: GlyphStyle) {
        let glyph = self
            .font
            .get_glyph(c)
            .unwrap_or(self.font.replacement_glyph());
        if glyph.is_rgba() {
            return self.render_emoji(c, x, y, style);
        }

        let orig_x = x * self.font.glyph_width() as usize;
        let orig_y = y * self.font.glyph_height() as usize;
        let px_width = self.glyph_size(c) * self.font.glyph_width() as usize;
        let px_height = self.font.glyph_height() as usize;
        let mut row_rgb = vec![0u32; px_width];

        let mut i = 0;
        let mut px_i = orig_y * self.width_px + orig_x;

        for px_y in 0..px_height {
            for px_x in 0..px_width {
                let glyph_alpha = glyph.data()[i];
                i += 1;

                let fg_color = Color {
                    r: (glyph_alpha as u16 * style.fg_color.r as u16 / 255)
                        as u8,
                    g: (glyph_alpha as u16 * style.fg_color.g as u16 / 255)
                        as u8,
                    b: (glyph_alpha as u16 * style.fg_color.b as u16 / 255)
                        as u8,
                };
                self.back_buffer[px_i] = fg_color.with_alpha(glyph_alpha);
                px_i += 1;

                let bg_color = style.bg_color.unwrap_or_else(|| {
                    let rgb = &self.background[(px_i * 4)..];
                    Color {
                        r: rgb[2],
                        g: rgb[1],
                        b: rgb[0],
                    }
                });
                let color = Color::blend(fg_color, glyph_alpha, bg_color);
                row_rgb[px_x] = unsafe { transmute(color.as_bgra()) };
            }

            self.fb.copy(orig_x, orig_y + px_y, &row_rgb);
            px_i += self.width_px - px_width;
        }
    }

    fn render_emoji(&mut self, c: char, x: usize, y: usize, style: GlyphStyle) {
        let glyph = self
            .font
            .get_glyph(c)
            .unwrap_or(self.font.replacement_glyph());

        let orig_x = x * self.font.glyph_width() as usize;
        let orig_y = y * self.font.glyph_height() as usize;

        for (i, rgba) in glyph.data().chunks_exact(4).enumerate() {
            let x = orig_x + i % (self.font.glyph_width() as usize * 2);
            let y = orig_y + i / (self.font.glyph_width() as usize * 2) + 2;
            let px_i = y * self.width_px + x;
            let fg_color = ColorA {
                r: rgba[2],
                g: rgba[1],
                b: rgba[0],
                a: rgba[3],
            };
            self.back_buffer[px_i] = fg_color;
            let bg_color = style
                .bg_color
                .unwrap_or_else(|| {
                    let rgb = &self.background[(px_i * 4)..];
                    Color {
                        r: rgb[2],
                        g: rgb[1],
                        b: rgb[0],
                    }
                })
                .with_alpha(255);
            let color = fg_color.blend(bg_color);
            self.fb.put(x, y, color.as_rgb());
        }
    }

    fn bg_color_at(&self, x: usize, y: usize) -> Color {
        let rgb = &self.background[((y * 1920 + x) * 4)..];

        Color {
            r: rgb[2],
            g: rgb[1],
            b: rgb[0],
        }
    }

    fn cell_at(&mut self, x: usize, y: usize) -> &mut TermCell {
        &mut self.cells[y * self.columns + x]
    }

    // TODO: does not handle escape commands in multiple parts.
    fn handle_escape(&mut self, s: &str) -> usize {
        let mut it = EscapeIterator::new(s);

        for cmd in &mut it {
            self.run_escape_command(cmd);
        }

        it.continuation_offset()
    }

    fn run_escape_command(&mut self, cmd: EscapeCommand) {
        use EscapeCommand::*;

        match cmd {
            SetFgColor(c) => self.curr_style.fg_color = c,
            ClearFgColor => self.curr_style.fg_color = DEFAULT_FG_COLOR,
            SetBgColor(c) => self.curr_style.bg_color = Some(c),
            ClearBgColor => self.curr_style.bg_color = None,
            Newline => {
                if self.cursor_x > 0 {
                    self.cursor_x = 0;
                    self.advance_y();
                }
            }
        }
    }
}

impl<Fb: FramebufferScreen> fmt::Write for Terminal<Fb> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s);
        Ok(())
    }
}

impl Default for TermCell {
    fn default() -> Self {
        Self {
            c: ' ',
            style: Default::default(),
        }
    }
}

impl Default for GlyphStyle {
    fn default() -> Self {
        Self {
            fg_color: DEFAULT_FG_COLOR,
            bg_color: None,
        }
    }
}

#[derive(Debug)]
pub enum EscapeCommand {
    SetFgColor(Color),
    ClearFgColor,
    SetBgColor(Color),
    ClearBgColor,
    Newline,
}

impl FromStr for EscapeCommand {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (cmd, arg) = s
            .split_once('=')
            .map(|(k, v)| (k, Some(v)))
            .unwrap_or((s, None));

        use EscapeCommand::*;
        Ok(match (cmd, arg) {
            ("fg", Some(arg)) => SetFgColor(arg.parse().map_err(|_| ())?),
            ("!fg", None) => ClearFgColor,
            ("bg", Some(arg)) => SetBgColor(arg.parse().map_err(|_| ())?),
            ("!bg", None) => ClearBgColor,
            ("nl", None) => Newline,
            _ => return Err(()),
        })
    }
}

pub struct EscapeIterator<'a> {
    s: Option<&'a str>,
    off: usize,
}

impl<'a> EscapeIterator<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { s: Some(s), off: 0 }
    }

    #[inline]
    pub fn continuation_offset(&self) -> usize {
        self.off
    }
}

impl Iterator for EscapeIterator<'_> {
    type Item = EscapeCommand;

    fn next(&mut self) -> Option<Self::Item> {
        let s = self.s?;

        if self.off == 0 {
            if s.len() == 0 || s.as_bytes()[0] != b'<' {
                return None;
            }
            if let Some(end_pos) = s.find('>') {
                self.off = end_pos + 1;
                self.s = Some(&s[1..end_pos]);
            } else {
                return None;
            }
        }

        loop {
            let s = self.s?;
            if let Some(pos) = s.find(';') {
                self.s = Some(&s[(pos + 1)..]);
                match s[..pos].parse() {
                    Ok(cmd) => break Some(cmd),
                    Err(_) => continue,
                }
            } else {
                break self.s.take().and_then(|s| s.parse().ok());
            }
        }
    }
}
