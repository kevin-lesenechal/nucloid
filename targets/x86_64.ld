/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

ENTRY(_start)

VA_BASE = 0xffff800000000000;
PA_BASE = 0x0000000000100000;

SECTIONS {
    . = VA_BASE + PA_BASE;

    .text ALIGN(4K) : AT(ADDR(.text) - VA_BASE) {
        KEEP(*(.multiboot))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - VA_BASE) {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - VA_BASE) {
        *(.data .data.*)
    }

    .bss ALIGN(4K) (NOLOAD) : AT(ADDR(.bss) - VA_BASE) {
        . = ALIGN(4K);
        *(.boot_page_tables)
        . = ALIGN(4K);
        *(.boot_stack)
        *(.bss .bss.*)
    }
}
