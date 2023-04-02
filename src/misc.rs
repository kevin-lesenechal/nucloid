/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use core::fmt;
use core::fmt::Formatter;
use core::ops::{BitAnd, Not};
use num_integer::Integer;

pub struct BinSize(pub u64);

impl fmt::Display for BinSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let size;
        let unit;

        if self.0 < 1024 {
            size = self.0 as f64; // TODO: ensure no FPU register is used
            unit = "o";
        } else if self.0 < 1024 * 1024 {
            size = self.0 as f64 / 1024.0;
            unit = "Kio";
        } else if self.0 < 1024 * 1024 * 1024 {
            size = self.0 as f64 / 1024.0 / 1024.0;
            unit = "Mio";
        } else {
            size = self.0 as f64 / 1024.0 / 1024.0 / 1024.0;
            unit = "Gio";
        }

        if unit == "o" {
            write!(f, "{} {}", size, unit)
        } else {
            write!(f, "{:.2} {}", size, unit)
        }
    }
}

/// Returns the next integer multiple of `multiple` or `n` if already a
/// multiple of `multiple`.
// TODO: make const (num_integer does not support it)
pub fn align_up<T>(n: T, multiple: T) -> T
    where T: Integer + Not<Output = T> + BitAnd<Output = T> + Copy
{
    (n + (multiple - T::one())) & !(multiple - T::one())
}

/// Return the bit position (counting from 0 from the LSB) of the first bit at
/// one from MSB to LSB. Return 0 if all zeroes.
pub fn first_bit_pos(n: usize) -> u8 {
    for i in (0..64).rev() {
        if (n & (1 << i)) > 0 {
            return i;
        }
    }

    0
}

#[cfg(test)]
mod test {
    use crate::misc::first_bit_pos;

    #[test]
    fn test_first_bit_pos() {
        assert_eq!(first_bit_pos(0b0001_0101), 4);
        assert_eq!(first_bit_pos(0b1001_0101), 7);
        assert_eq!(first_bit_pos(0b11100000_10010101), 15);
        assert_eq!(first_bit_pos(0), 0);
    }
}

#[macro_use]
pub mod macros {
    // Author: https://users.rust-lang.org/u/ExpHP
    #[repr(C)] // guarantee 'bytes' comes after '_align'
    pub struct AlignedAs<Align, Bytes: ?Sized> {
        pub _align: [Align; 0],
        pub bytes: Bytes,
    }

    macro_rules! include_bytes_align_as {
        ($align_ty:ty, $path:expr) => {
            {  // const block expression to encapsulate the static
                use $crate::misc::macros::AlignedAs;

                // this assignment is made possible by CoerceUnsized
                static ALIGNED: &AlignedAs::<$align_ty, [u8]> = &AlignedAs {
                    _align: [],
                    bytes: *include_bytes!($path),
                };

                &ALIGNED.bytes
            }
        };
    }
}
