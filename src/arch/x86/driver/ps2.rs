use x86::io::{inb, outb};

use crate::arch::sync::{pop_critical_region, push_critical_region};
use crate::driver::keyboard::{Key, KeyEvent, on_key_event};
use crate::sync::Spinlock;

const DATA_PORT: u16 = 0x60;
const STATUS_REGISTER: u16 = 0x64;
const COMMAND_REGISTER: u16 = 0x64;

const CMD_READ_CONF: u8 = 0x20;
const CMD_WRITE_CONF: u8 = 0x20;
const CMD_DISABLE_DEV1: u8 = 0xae;
const CMD_DISABLE_DEV2: u8 = 0xa7;
const CMD_ENABLE_DEV1: u8 = 0xae;

const STATUS_OUTPUT_BUSY: u8 = 1 << 0;
const STATUS_INPUT_BUSY: u8 = 1 << 1;

const CTRL_CONF_DEV1_INTERRUPT: u8 = 1 << 0;
const CTRL_CONF_DEV2_INTERRUPT: u8 = 1 << 1;
const CTRL_CONF_DEV1_TRANSLATION: u8 = 1 << 6;

static PS2_KEYBOARD: Spinlock<Option<PS2Keyboard>> = Spinlock::new(None);

pub struct PS2Keyboard {
    is_e0_state: bool,
}

impl PS2Keyboard {
    pub fn new() -> Self {
        Self {
            is_e0_state: false,
        }
    }

    pub fn on_irq(&mut self) {
        if is_output_full() {
            let byte = unsafe { inb(DATA_PORT) };
            if byte == 0xe0 {
                self.is_e0_state = true;
            } else {
                let ev = self.read_key(byte);
                self.is_e0_state = false;
                on_key_event(ev);
            }
        }
    }

    fn read_key(&self, key: u8) -> KeyEvent {
        let pressed = key & 1 << 7 == 0;
        let key = key & !(1 << 7);

        let key = if self.is_e0_state {
            match key {
                0x1d => Key::RightCtrl,
                0x38 => Key::AltGr,
                0x5b => Key::LeftMeta,
                0x5c => Key::RightMeta,
                0x5d => Key::Menu,

                0x52 => Key::Insert,
                0x53 => Key::Del,
                0x47 => Key::Home,
                0x4f => Key::End,
                0x49 => Key::PgUp,
                0x51 => Key::PgDown,
                0x48 => Key::Up,
                0x50 => Key::Down,
                0x4b => Key::Left,
                0x4d => Key::Right,

                0x1c => Key::KeypadEnter,
                0x35 => Key::KeypadDiv,

                _ => return KeyEvent::Unknown,
            }
        } else {
            match key {
                0x01 => Key::Escape,
                0x0e => Key::Backspace,
                0x39 => Key::Space,
                0x1c => Key::Enter,
                0x0f => Key::Tab,
                0x3a => Key::CapsLock,

                0x2a => Key::LeftShift,
                0x36 => Key::RightShift,
                0x1d => Key::LeftCtrl,
                0x38 => Key::Alt,

                0x3b..=0x44 => Key::F(key - 0x3b + 1),
                0x57 => Key::F(11),
                0x58 => Key::F(12),

                0x46 => Key::ScrollLock,

                0x29 => Key::Backquote,
                0x02..=0x0a => Key::Digit(key - 0x02 + 1),
                0x0b => Key::Digit(0),
                0x0c => Key::Dash,
                0x0d => Key::Equal,

                0x52 => Key::KeypadDigit(0),
                0x4f => Key::KeypadDigit(1),
                0x50 => Key::KeypadDigit(2),
                0x51 => Key::KeypadDigit(3),
                0x4b => Key::KeypadDigit(4),
                0x4c => Key::KeypadDigit(5),
                0x4d => Key::KeypadDigit(6),
                0x47 => Key::KeypadDigit(7),
                0x48 => Key::KeypadDigit(8),
                0x49 => Key::KeypadDigit(9),
                0x45 => Key::KeypadNumLock,
                0x53 => Key::KeypadPeriod,
                0x4e => Key::KeypadPlus,
                0x4a => Key::KeypadMinus,
                0x37 => Key::KeypadMul,

                0x10 => Key::Letter('Q'),
                0x11 => Key::Letter('W'),
                0x12 => Key::Letter('E'),
                0x13 => Key::Letter('R'),
                0x14 => Key::Letter('T'),
                0x15 => Key::Letter('Y'),
                0x16 => Key::Letter('U'),
                0x17 => Key::Letter('I'),
                0x18 => Key::Letter('O'),
                0x19 => Key::Letter('P'),

                0x1e => Key::Letter('A'),
                0x1f => Key::Letter('S'),
                0x20 => Key::Letter('D'),
                0x21 => Key::Letter('F'),
                0x22 => Key::Letter('G'),
                0x23 => Key::Letter('H'),
                0x24 => Key::Letter('J'),
                0x25 => Key::Letter('K'),
                0x26 => Key::Letter('L'),

                0x2c => Key::Letter('Z'),
                0x2d => Key::Letter('X'),
                0x2e => Key::Letter('C'),
                0x2f => Key::Letter('V'),
                0x30 => Key::Letter('B'),
                0x31 => Key::Letter('N'),
                0x32 => Key::Letter('M'),

                _ => return KeyEvent::Unknown,
            }
        };

        if pressed {
            KeyEvent::Pressed(key)
        } else {
            KeyEvent::Released(key)
        }
    }
}

pub fn init() {
    push_critical_region();

    drain_output();

    send_cmd(CMD_DISABLE_DEV1);
    send_cmd(CMD_DISABLE_DEV2);

    let mut ctrl = read_conf_byte(0);
    ctrl &= !CTRL_CONF_DEV1_INTERRUPT;
    ctrl &= !CTRL_CONF_DEV2_INTERRUPT;
    ctrl &= !CTRL_CONF_DEV1_TRANSLATION;
    write_conf_byte(0, ctrl);

    send_cmd(CMD_ENABLE_DEV1);

    drain_output();

    *PS2_KEYBOARD.lock() = Some(PS2Keyboard::new());

    pop_critical_region();
}

pub fn on_irq() {
    if let Some(kb) = PS2_KEYBOARD.lock().as_mut() {
        kb.on_irq();
    }
}

pub fn hard_reset() -> ! {
    unsafe {
        outb(COMMAND_REGISTER, 0xfe);
    }

    unreachable!()
}

fn read_conf_byte(offset: u8) -> u8 {
    if offset > 17 {
        panic!("Invalid offset");
    }

    send_cmd(CMD_READ_CONF + offset);
    wait_for_output();
    unsafe {
        inb(DATA_PORT)
    }
}

fn write_conf_byte(offset: u8, byte: u8) {
    if offset > 17 {
        panic!("Invalid offset");
    }
    wait_input_ready();
    unsafe {
        outb(DATA_PORT, byte);
    }
    send_cmd(CMD_WRITE_CONF + offset);
}

fn send_cmd(cmd: u8) {
    wait_input_ready();
    unsafe {
        outb(COMMAND_REGISTER, cmd);
    }
}

fn wait_input_ready() {
    while unsafe { inb(STATUS_REGISTER) } & STATUS_INPUT_BUSY > 0 {}
}

fn wait_for_output() {
    while !is_output_full() {}
}

fn is_output_full() -> bool {
    (unsafe { inb(STATUS_REGISTER) } & STATUS_OUTPUT_BUSY) > 0
}

fn drain_output() {
    while is_output_full() {
        unsafe {
            inb(DATA_PORT);
        }
    }
}
