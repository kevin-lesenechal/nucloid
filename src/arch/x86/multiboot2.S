/******************************************************************************
 * Copyright © 2021 Kévin Lesénéchal <kevin.lesenechal@gmail.com>             *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 **************************************************************************** */

.set MAGIC,         0xe85250d6
.set ARCH,          0 # IA-32
.set HEADER_SIZE,   24 # Multiboot2 header size (octets)

.section .multiboot2
    .long   MAGIC
    .long   ARCH
    .long   HEADER_SIZE
    .long   -(MAGIC + ARCH + HEADER_SIZE)

    # End tag
    .short  0 # Type
    .short    # Flags
    .long   8 # Size
