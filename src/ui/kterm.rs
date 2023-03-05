use core::fmt;
use core::fmt::Write;

use crate::arch::VesaFramebuffer;
use crate::sync::Spinlock;
use crate::ui::term::Terminal;

pub static KERNEL_TERMINAL: Spinlock<Option<Terminal<VesaFramebuffer>>>
    = Spinlock::new(None);

pub fn _print(args: fmt::Arguments) {
    let mut kterm = KERNEL_TERMINAL.lock();
    if let Some(ref mut kterm) = *kterm {
        let _ = kterm.write_fmt(args);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::ui::kterm::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! dbg {
    // NOTE: We cannot use `concat!` to make a static string as a format argument
    // of `eprintln!` because `file!` could contain a `{` or
    // `$val` expression could be a block (`{ .. }`), in which case the `eprintln!`
    // will be malformed.
    () => {
        $crate::debug!("[{}:{}]", core::file!(), core::line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::debug!("[{}:{}] {} = {:#?}",
                    core::file!(), core::line!(), core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
