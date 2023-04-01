use crate::{arch, print, println, warning};
use crate::sync::Spinlock;

#[derive(Debug)]
pub enum KeyEvent {
    Unknown,
    Pressed(Key),
    Released(Key),
}

#[derive(Debug)]
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

static KEYBOARD: Spinlock<Option<Keyboard>> = Spinlock::new(None);

struct Keyboard {
    lctrl: bool,
    rctrl: bool,
    lshift: bool,
    rshift: bool,
    alt: bool,
    altgr: bool,
    lmeta: bool,
    rmeta: bool,
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
                    Key::Letter(l) => {
                        print!("{}", if self.has_shift() { l } else { l.to_ascii_lowercase() })
                    },
                    Key::Digit(n) | Key::KeypadDigit(n) => print!("{}", (0x30 + n) as char),
                    Key::Space => print!(" "),
                    Key::Backquote => print!("`"),
                    Key::Dash => print!("-"),
                    Key::Equal => print!("="),
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
                    _ => (),
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
