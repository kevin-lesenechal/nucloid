use core::str::FromStr;

use crate::{arch, print, println, warning};
use crate::sync::Spinlock;
use crate::ui::keymap::{Keymap, KeymapState};
use crate::ui::kterm::KERNEL_TERMINAL;

#[derive(Debug)]
pub enum KeyEvent {
    Unknown,
    Pressed(Key),
    Released(Key),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Key {
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftMeta,
    RightMeta,
    Alt,
    AltGr,
    Menu,

    Backquote,
    Digit(u8),
    Dash,
    Equal,
    Backslash,
    LeftBracket,
    RightBracket,
    Semicolon,
    SingleQuote,
    Comma,
    Period,
    Slash,
    Iso,

    Letter(char),

    Tab,
    CapsLock,
    Space,
    Enter,
    Escape,
    Backspace,
    F(u8),

    Insert,
    Del,
    Home,
    End,
    PgUp,
    PgDown,
    Up,
    Down,
    Left,
    Right,

    ScrollLock,

    KeypadDigit(u8),
    KeypadPeriod,
    KeypadEnter,
    KeypadPlus,
    KeypadMinus,
    KeypadMul,
    KeypadDiv,
    KeypadNumLock,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Deadkey {
    GraveAccent,
    AcuteAccent,
    Circumflex,
    Tilde,
    Macron,
    Breve,
    Diaeresis,
    Ring,
    Caron,
}

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.len() == 1 {
            let c = s.chars().nth(0).unwrap();
            match c {
                '`' => Key::Backquote,
                '0'..='9' => Key::Digit(c as u8 - 0x30),
                '-' => Key::Dash,
                '=' => Key::Equal,
                '[' => Key::LeftBracket,
                ']' => Key::RightBracket,
                ';' => Key::Semicolon,
                '\'' => Key::SingleQuote,
                ',' => Key::Comma,
                '.' => Key::Period,
                '/' => Key::Slash,
                '\\' => Key::Backslash,
                'A'..='Z' => Key::Letter(c),
                _ => return Err(()),
            }
        } else {
            match s {
                "iso" => Key::Iso,
                "KP0" => Key::KeypadDigit(0),
                "KP1" => Key::KeypadDigit(1),
                "KP2" => Key::KeypadDigit(2),
                "KP3" => Key::KeypadDigit(3),
                "KP4" => Key::KeypadDigit(4),
                "KP5" => Key::KeypadDigit(5),
                "KP6" => Key::KeypadDigit(6),
                "KP7" => Key::KeypadDigit(7),
                "KP8" => Key::KeypadDigit(8),
                "KP9" => Key::KeypadDigit(9),
                "KP." => Key::KeypadPeriod,
                "KP+" => Key::KeypadPlus,
                "KP-" => Key::KeypadMinus,
                "KP*" => Key::KeypadMul,
                "KP/" => Key::KeypadDiv,
                _ => return Err(()),
            }
        })
    }
}

impl Deadkey {
    pub fn apply(&self, c: char) -> Option<char> {
        Some(match self {
            Deadkey::Circumflex => {
                match c {
                    'a' => 'â',
                    'z' => 'ẑ',
                    'e' => 'ê',
                    'y' => 'ŷ',
                    'u' => 'û',
                    'i' => 'î',
                    'o' => 'ô',
                    's' => 'ŝ',
                    'g' => 'ĝ',
                    'h' => 'ĥ',
                    'j' => 'ĵ',
                    'w' => 'ŵ',
                    'c' => 'ĉ',
                    'A' => 'Â',
                    'Z' => 'Ẑ',
                    'E' => 'Ê',
                    'Y' => 'Ŷ',
                    'U' => 'Û',
                    'I' => 'Î',
                    'O' => 'Ô',
                    'S' => 'Ŝ',
                    'G' => 'Ĝ',
                    'H' => 'Ĥ',
                    'J' => 'Ĵ',
                    'W' => 'Ŵ',
                    'C' => 'Ĉ',
                    _ => return None,
                }
            },
            Deadkey::Diaeresis => {
                match c {
                    'a' => 'ä',
                    'e' => 'ë',
                    't' => 'ẗ',
                    'y' => 'ÿ',
                    'u' => 'ü',
                    'i' => 'ï',
                    'o' => 'ö',
                    'h' => 'ḧ',
                    'w' => 'ẅ',
                    'x' => 'ẍ',
                    _ => return None,
                }
            },
            // TODO: Implement the rest
            _ => return None,
        })
    }

    pub fn as_standalone(&self) -> Option<char> {
        Some(match self {
            Deadkey::GraveAccent => '`',
            Deadkey::AcuteAccent => '´',
            Deadkey::Circumflex => '^',
            Deadkey::Tilde => '~',
            Deadkey::Macron => '¯',
            Deadkey::Breve => '˘',
            Deadkey::Diaeresis => '¨',
            Deadkey::Ring => '°',
            Deadkey::Caron => 'ˇ',
        })
    }
}

impl TryFrom<char> for Deadkey {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        Ok(match value {
            '\u{0300}' => Deadkey::GraveAccent,
            '\u{0301}' => Deadkey::AcuteAccent,
            '\u{0302}' => Deadkey::Circumflex,
            '\u{0303}' => Deadkey::Tilde,
            '\u{0304}' => Deadkey::Macron,
            '\u{0306}' => Deadkey::Breve,
            '\u{0308}' => Deadkey::Diaeresis,
            '\u{030a}' => Deadkey::Ring,
            '\u{030c}' => Deadkey::Caron,
            _ => return Err(()),
        })
    }
}

static KEYBOARD: Spinlock<Option<Keyboard>> = Spinlock::new(None);

struct Keyboard {
    keymap: KeymapState,

    lctrl: bool,
    rctrl: bool,
    lshift: bool,
    rshift: bool,
    alt: bool,
    altgr: bool,
    lmeta: bool,
    rmeta: bool,
    capslock: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            lctrl: false,
            rctrl: false,
            lshift: false,
            rshift: false,
            alt: false,
            altgr: false,
            lmeta: false,
            rmeta: false,
            capslock: false,
            keymap: KeymapState::new(Keymap::from_file(include_bytes!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/media/fr.keymap")
            )).unwrap()),
        }
    }

    pub fn has_ctrl(&self) -> bool {
        self.lctrl || self.rctrl
    }

    pub fn has_shift(&self) -> bool {
        self.lshift || self.rshift
    }

    pub fn has_meta(&self) -> bool {
        self.lmeta || self.rmeta
    }

    pub fn on_key_event(&mut self, event: KeyEvent) {
        match event {
            KeyEvent::Pressed(key) =>
                match key {
                    Key::Space => print!(" "),
                    Key::Enter | Key::KeypadEnter => println!(),
                    Key::ScrollLock => arch::cpu::reset(),

                    Key::LeftShift => self.lshift = true,
                    Key::RightShift => self.rshift = true,
                    Key::LeftCtrl => self.lctrl = true,
                    Key::RightCtrl => self.rctrl = true,
                    Key::Alt => self.alt = true,
                    Key::AltGr => self.altgr = true,
                    Key::LeftMeta => self.lmeta = true,
                    Key::RightMeta => self.rmeta = true,
                    Key::CapsLock => self.capslock = !self.capslock, // TODO: LED

                    _ => {
                        if self.has_ctrl() {
                            match key {
                                Key::Letter('L') => KERNEL_TERMINAL.lock().as_mut().unwrap().clear(),
                                _ => (),
                            }
                            return;
                        }

                        let c = self.keymap.glyph(
                            key,
                            self.altgr,
                            self.capslock,
                            self.has_shift()
                        );
                        if let Some(c) = c {
                            print!("{c}");
                        }
                    },
                },
            KeyEvent::Released(key) =>
                match key {
                    Key::LeftShift => self.lshift = false,
                    Key::RightShift => self.rshift = false,
                    Key::LeftCtrl => self.lctrl = false,
                    Key::RightCtrl => self.rctrl = false,
                    Key::Alt => self.alt = false,
                    Key::AltGr => self.altgr = false,
                    Key::LeftMeta => self.lmeta = false,
                    Key::RightMeta => self.rmeta = false,
                    _ => (),
                },
            _ => (),
        }
    }
}

pub fn init() {
    *KEYBOARD.lock() = Some(Keyboard::new());
}

pub fn on_key_event(event: KeyEvent) {
    if let Some(kb) = KEYBOARD.lock().as_mut() {
        kb.on_key_event(event);
    } else {
        warning!("key event with no kernel keyboard");
    }
}
