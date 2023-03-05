/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use crate::sync::Spinlock;

use core::fmt;

pub static DEFAULT_LOGGER: Spinlock<&'static mut (dyn Logger + Send)>
    = Spinlock::new(unsafe { &mut NULL_LOGGER });

pub trait Logger {
    fn log(&mut self, severity: Severity, args: fmt::Arguments);
}

pub enum Severity {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl Severity {
    pub fn label(&self) -> &str {
        use Severity::*;

        match self {
            Debug       => "debug",
            Info        => "info",
            Notice      => "notice",
            Warning     => "warn",
            Error       => "error",
            Critical    => "critical",
            Alert       => "alert",
            Emergency   => "emerg",
        }
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Debug, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Info, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! notice {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Notice, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Warning, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Error, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Critical, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Alert, format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! emergency {
    ($($arg:tt)*) => ({
        let mut logger = $crate::logging::DEFAULT_LOGGER.lock();
        logger.log($crate::logging::Severity::Emergency, format_args!($($arg)*));
    });
}

struct NullLogger;

static mut NULL_LOGGER: NullLogger = NullLogger;

impl Logger for NullLogger {
    fn log(&mut self, _severity: Severity, _args: fmt::Arguments) {
    }
}
