use core::fmt;
use core::fmt::Write;
use std::print;
use crate::logging::{DEFAULT_LOGGER, Logger, Severity};

pub struct SerialDevice;

pub static mut LOGGER_SERIAL: Option<SerialDevice> = Some(SerialDevice);

impl fmt::Write for SerialDevice {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print!("{}", s);

        Ok(())
    }
}

impl Logger for SerialDevice {
    fn log(&mut self, severity: Severity, args: fmt::Arguments) {
        let (color, severity_str) = match severity {
            Severity::Debug => ("\x1b[90m", "debug"),
            Severity::Info => ("\x1b[37m", "info"),
            Severity::Notice => ("\x1b[97m", "notice"),
            Severity::Warning => ("\x1b[93m", "warning"),
            Severity::Error => ("\x1b[31m", "error"),
            Severity::Critical => ("\x1b[1;31m", "critic."),
            Severity::Alert => ("\x1b[1;97;41m", "ALERT"),
            Severity::Emergency => ("\x1b[1;93;41m", "EMERG."),
        };

        write!(self, "{}{:>7}: ", color, severity_str).unwrap();
        self.write_fmt(args).unwrap();
        write!(self, "\x1b[0m\n").unwrap();
    }
}

#[ctor::ctor]
fn init() {
    *DEFAULT_LOGGER.lock() = unsafe { LOGGER_SERIAL.as_mut().unwrap() };
}
