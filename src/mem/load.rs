/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::ops::Range;

use crate::mem::VAddr;

unsafe extern "C" {
    /// The virtual address at which the kernel image, as loaded by the
    /// bootloader, resides; the address is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_start: u8;

    /// The address of the first byte past the kernel image in virtual memory.
    /// The address is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_end: u8;

    /// The numbers of bytes of the kernel image, including padding. The size
    /// is guaranteed to be page-aligned.
    /// The value is passed as a symbol, i.e. a memory address, what this
    /// address points to is irrelevant; ONLY take the ADDRESS of this variable
    /// and *IN NO CASE ACCESS THE VALUE EVEN FOR READING*.
    static __kernel_image_size: u8;

    static __kernel_text_start: u8;

    static __kernel_text_end: u8;

    static __kernel_rodata_start: u8;

    static __kernel_rodata_end: u8;
}

#[inline]
pub fn kernel_image() -> Range<VAddr> {
    unsafe {
        VAddr(&__kernel_image_start as *const u8 as usize)
            ..VAddr(&__kernel_image_end as *const u8 as usize)
    }
}

#[inline]
pub fn kernel_text_segment() -> Range<VAddr> {
    unsafe {
        VAddr(&__kernel_text_start as *const u8 as usize)
            ..VAddr(&__kernel_text_end as *const u8 as usize)
    }
}

#[inline]
pub fn kernel_rodata_segment() -> Range<VAddr> {
    unsafe {
        VAddr(&__kernel_rodata_start as *const u8 as usize)
            ..VAddr(&__kernel_rodata_end as *const u8 as usize)
    }
}
