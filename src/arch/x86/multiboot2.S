/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 **************************************************************************** */

.set MAGIC,         0xe85250d6
.set ARCH,          0           # IA-32
.set HEADER_SIZE,   44          # Multiboot2 header size (octets)

#ifdef __x86_64__
#  define VA_BASE 0xffff800000000000
#else
#  define VA_BASE 0xc0000000
#endif

.section .multiboot2
    .long   MAGIC
    .long   ARCH
    .long   HEADER_SIZE
    .long   (-(MAGIC + ARCH + HEADER_SIZE)) & 0xffffffff

    # Entry address
    .short  3       # Type
    .short  0       # Flags
    .long   12      # Size
    .long   _start - VA_BASE
    .skip   4       # Align to 8 octets

    # Framebuffer
    .short  5       # Type
    .short  0       # Flags
    .long   20      # Size
    .long   1920    # Width
    .long   1080    # Height
    .long   32      # Depth
    .skip   4       # Align to 8 octets

    # End tag
    .short  0       # Type
    .short  0       # Flags
    .long   8       # Size
