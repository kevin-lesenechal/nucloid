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
use core::cell::RefCell;
use core::fmt;
use core::str::FromStr;

use crate::driver::screen::{Color, FramebufferScreen};
use crate::ui::pxfont::PxFont;

const DEFAULT_FG_COLOR: Color = Color { r: 169, g: 183, b: 198 };

pub struct Terminal<Fb> {
    background: &'static [u8],
    font: PxFont,
    fb: RefCell<Fb>,
    width_px: usize,
    height_px: usize,
    columns: usize,
    rows: usize,
    cursor_x: usize,
    cursor_y: usize,
    curr_style: GlyphStyle,
    cells: VecDeque<TermCell>,
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
        let font = PxFont::from_data(include_bytes!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/media/iosevka.pxfont")
        )).unwrap();
        let columns = width_px / font.glyph_width() as usize;
        let rows = height_px / font.glyph_height() as usize;

        let mut term = Self {
            background: include_bytes_align_as!(u32,
                concat!(env!("CARGO_MANIFEST_DIR"), "/media/wallpaper.data")
            ),
            font,
            fb: RefCell::new(fb),
            width_px,
            height_px,
            columns,
            rows,
            cursor_x: 0,
            cursor_y: 0,
            curr_style: Default::default(),
            cells: VecDeque::new(),
        };
        term.clear();

        term
    }

    pub fn clear(&mut self) {
        self.clear_visual();

        self.cells = vec![Default::default(); self.rows * self.columns].into();
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn clear_visual(&self) {
        let mut fb = self.fb.borrow_mut();

        let dst_row_len = fb.dimensions().0;
        let mut src_row = unsafe { self.background.align_to::<u32>().1 };

        for y in 0..fb.dimensions().1 {
            fb.copy(0, y, unsafe { (&src_row[..dst_row_len]).align_to::<u32>().1 });
            src_row = &src_row[1920..];
        }
    }

    pub fn write(&mut self, s: &str) {
        let mut it = s.char_indices();

        while let Some((i, c)) = it.next() {
            if c == '\x1b' {
                let _ = it.advance_by(self.handle_escape(&s[(i + 1)..]));
            }
            self.putc(c);
        }
    }

    pub fn putc(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.advance_y();
            },
            '\t' => self.advance_x(8 - (self.cursor_x & 0b111)),
            //' ' | '\u{a0}' | '\u{202f}' => self.advance_x(1),
            '\r' => self.cursor_x = 0,
            '\x00'..='\x1f' | '\x7f' => return,
            c => {
                let glyph_size = self.glyph_size(c);
                if self.cursor_x + glyph_size >= self.columns {
                    self.cursor_x = 0;
                    self.advance_y();
                }
                self.render_glyph(
                    c,
                    self.cursor_x,
                    self.cursor_y,
                    self.curr_style,
                );
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
            },
        }
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

        self.rerender();

        self.cursor_y = self.cursor_y.saturating_sub(nr_lines);
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
        self.font.get_glyph(c)
            .unwrap_or(self.font.replacement_glyph())
            .nr_columns()
    }

    fn render_glyph(
        &self,
        c: char,
        x: usize,
        y: usize,
        style: GlyphStyle,
    ) {
        let glyph = self.font.get_glyph(c)
            .unwrap_or(self.font.replacement_glyph());
        if glyph.is_rgba() {
            return self.render_emoji(c, x, y, style);
        }

        let orig_x = x * self.font.glyph_width() as usize;
        let orig_y = y * self.font.glyph_height() as usize;
        let nr_cols = self.glyph_size(c) * self.font.glyph_width() as usize;

        let mut fb = self.fb.borrow_mut();

        for (i, &value) in glyph.data().into_iter().enumerate() {
            let x = orig_x + i % nr_cols as usize;
            let y = orig_y + i / nr_cols as usize;
            let fg_color = Color {
                r: (value as u16 * style.fg_color.r as u16 / 255) as u8,
                g: (value as u16 * style.fg_color.g as u16 / 255) as u8,
                b: (value as u16 * style.fg_color.b as u16 / 255) as u8,
            };
            let bg_color = style.bg_color
                .unwrap_or_else(|| self.bg_color_at(x, y));
            let color = Color::blend(fg_color, value, bg_color);
            fb.put(x, y, color);
        }
    }

    fn render_emoji(
        &self,
        c: char,
        x: usize,
        y: usize,
        style: GlyphStyle,
    ) {
        let glyph = self.font.get_glyph(c)
            .unwrap_or(self.font.replacement_glyph());

        let orig_x = x * self.font.glyph_width() as usize;
        let orig_y = y * self.font.glyph_height() as usize;

        let mut fb = self.fb.borrow_mut();

        for (i, rgba) in glyph.data().chunks_exact(4).enumerate() {
            let x = orig_x + i % (self.font.glyph_width() as usize * 2);
            let y = orig_y + i / (self.font.glyph_width() as usize * 2) + 2;
            let fg_color = Color {
                r: rgba[2],
                g: rgba[1],
                b: rgba[0],
            };
            let bg_color = style.bg_color
                .unwrap_or_else(|| self.bg_color_at(x, y));
            let color = Color::blend(fg_color, rgba[3], bg_color);
            fb.put(x, y, color);
        }
    }

    fn bg_color_at(&self, x: usize, y: usize) -> Color {
        let rgb = &self.background[((y * 1920 + x) * 4)..];

        Color { r: rgb[2], g: rgb[1], b: rgb[0] }
    }

    fn rerender(&mut self) {
        self.clear_visual();

        for (i, cell) in self.cells.iter().enumerate() {
            let y = i / self.columns;
            let x = i % self.columns;

            if cell.c != '\0' && cell.c != ' ' {
                self.render_glyph(cell.c, x, y, cell.style);
            }
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
        let (cmd, arg) = s.split_once('=')
            .map(|(k, v)| (k, Some(v)))
            .unwrap_or((s, None));

        use EscapeCommand::*;
        Ok(match (cmd, arg) {
            ("fg", Some(arg)) =>
                SetFgColor(arg.parse().map_err(|_| ())?),
            ("!fg", None) => ClearFgColor,
            ("bg", Some(arg)) =>
                SetBgColor(arg.parse().map_err(|_| ())?),
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
        Self {
            s: Some(s),
            off: 0,
        }
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
