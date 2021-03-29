use core::str::FromStr;

use crate::driver::screen::{Color, FramebufferScreen};
use crate::ui::pxfont::PxFont;

const DEFAULT_FG_COLOR: Color = Color { r: 255, g: 255, b: 255 };

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
    fg_color: Color,
    bg_color: Option<Color>,
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
            background: include_bytes!(
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
            fg_color: DEFAULT_FG_COLOR,
            bg_color: None,
        };
        term.clear();

        term
    }

    pub fn clear(&mut self) {
        for y in 0..self.fb.dimensions().1 {
            for x in 0..self.fb.dimensions().0 {
                let rgb = &self.background[((y * 1920 + x) * 3)..];
                self.fb.put(x, y, Color { r: rgb[0], g: rgb[1], b: rgb[2] });
            }
        }
    }

    pub fn write(&mut self, s: &str) {
        let mut it = s.char_indices();//s.chars().enumerate();

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
                self.cursor_y += 1;
                self.cursor_x = 0;
                return;
            },
            '\t' => {
                self.cursor_x = (self.cursor_x & !0b111) + 7;
            },
            ' ' | '\u{a0}' | '\u{202f}' => (),
            '\r' => {
                self.cursor_x = 0;
                return;
            },
            '\x00'..='\x1f' | '\x7f' => return,
            c => {
                let glyph = self.font.get_glyph(c)
                    .unwrap_or(self.font.replacement_glyph());
                let orig_x = self.cursor_x * self.font.glyph_width() as usize;
                let orig_y = self.cursor_y * self.font.glyph_height() as usize;

                for (i, &value) in glyph.rgb_data().into_iter().enumerate() {
                    let x = orig_x + i % self.font.glyph_width() as usize;
                    let y = orig_y + i / self.font.glyph_width() as usize;
                    let fg_color = Color {
                        r: (value as u16 * self.fg_color.r as u16 / 255) as u8,
                        g: (value as u16 * self.fg_color.g as u16 / 255) as u8,
                        b: (value as u16 * self.fg_color.b as u16 / 255) as u8,
                    };
                    let bg_color = self.bg_color_at(x, y);
                    let color = Color::blend(fg_color, value, bg_color);
                    self.fb.put(x, y, color);
                }
            }
        }

        self.cursor_x += 1;
        if self.cursor_x >= self.columns {
            self.cursor_x = 0;
            self.cursor_y += 1;
        }
    }

    pub fn fg_color(&mut self, color: Color) {
        self.fg_color = color;
    }

    fn bg_color_at(&self, x: usize, y: usize) -> Color {
        let rgb = &self.background[((y * 1920 + x) * 3)..];

        Color { r: rgb[0], g: rgb[1], b: rgb[2] }
    }

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
            SetFgColor(c) => self.fg_color = c,
            ClearFgColor => self.fg_color = DEFAULT_FG_COLOR,
            SetBgColor(c) => self.bg_color = Some(c),
            ClearBgColor => self.bg_color = None,
        }
    }
}

#[derive(Debug)]
pub enum EscapeCommand {
    SetFgColor(Color),
    ClearFgColor,
    SetBgColor(Color),
    ClearBgColor,
}

impl FromStr for EscapeCommand {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (cmd, arg) = s.split_once('=')
            .map(|(k, v)| (k, Some(v)))
            .unwrap_or((s, None));

        use EscapeCommand::*;
        Ok(match (cmd, arg) {
            ("fg", Some(arg)) => {
                if arg == "!" {
                    ClearFgColor
                } else {
                    SetFgColor(arg.parse().map_err(|_| ())?)
                }
            },
            ("bg", Some(arg)) => {
                if arg == "!" {
                    ClearBgColor
                } else {
                    SetBgColor(arg.parse().map_err(|_| ())?)
                }
            },
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
            if s.len() == 0 || s.as_bytes()[0] != b'{' {
                return None;
            }
            if let Some(end_pos) = s.find('}') {
                self.off = end_pos + 1;
                self.s = Some(&s[1..end_pos]);
            } else {
                return None;
            }
        }

        let s = self.s?;

        if let Some(pos) = s.find(';') {
            let cmd = s[..pos].parse().ok()?;
            self.s = Some(&s[(pos + 1)..]);
            Some(cmd)
        } else {
            self.s.take().and_then(|s| s.parse().ok())
        }
    }
}
