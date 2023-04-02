/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

/*#[cfg(all(
    //target_arch = "x86_64",
    not(test)
))]*/
mod x86;

/*#[cfg(all(
    //target_arch = "x86_64",
    not(test)
))]*/
pub use crate::arch::x86::export::*;

/*#[cfg(test)]
mod test;

#[cfg(test)]
pub use crate::arch::test::export::*;*/
