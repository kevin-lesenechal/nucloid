use crate::arch::x86::driver::serial::SerialDevice;

pub static mut LOGGER_SERIAL: Option<SerialDevice> = None;
