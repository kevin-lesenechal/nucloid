/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

ENTRY(_start)

VA_BASE = 0xc0000000;
PA_BASE = 0x00100000;

SECTIONS {
    . = VA_BASE + PA_BASE;
    __kernel_image_start = .;

    __kernel_text_start = .;
    .text ALIGN(4K) : AT(ADDR(.text) - VA_BASE) {
        KEEP(*(.multiboot2))
        *(.text .text.*)
    }
    . = ALIGN(4K);
    __kernel_text_end = .;

    __kernel_rodata_start = .;
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - VA_BASE) {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K);
    __kernel_rodata_end = .;

    __kernel_data_start = .;
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
    . = ALIGN(4K);
    __kernel_data_end = .;

    __kernel_image_end = .;
    __kernel_image_size = __kernel_image_end - __kernel_image_start;
}
