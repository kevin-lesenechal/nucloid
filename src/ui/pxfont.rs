/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use alloc::vec::Vec;
use core::char::REPLACEMENT_CHARACTER;
use core::convert::TryInto;
use binrw::BinRead;
use binrw::io::{Cursor, Seek, SeekFrom};
use hashbrown::HashMap;
use thiserror_no_std::Error;

pub struct PxFont {
    chars: HashMap<char, Glyph>,
    glyph_width: u8,
    glyph_height: u8,
}

pub struct Glyph {
    px: Vec<u8>,
    nr_cols: u8,
    is_rgba: bool,
}

#[derive(Error, Debug)]
pub enum PxFontError {
    #[error("invalid PXFONT file header: {0}")]
    InvalidHeader(#[source] binrw::error::Error),

    #[error("invalid glyph block header: {0}")]
    InvalidGlyphBlockHeader(#[source] binrw::error::Error),

    #[error("invalid point code range {0}..={1} in glyph block")]
    InvalidGlyphBlockRange(u32, u32),

    #[error("invalid glyph {0:?}")]
    InvalidGlyph(char),

    #[error("the replacement glyph '�' is missing")]
    MissingReplacementGlyph,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"PXFONT")]
struct FileHeader {
    width: u8,
    height: u8,
}

#[derive(BinRead, Debug)]
#[br(little)]
struct GlyphBlock {
    start: u32,
    end: u32,
    rgba: u8,
}

impl PxFont {
    pub fn from_data(data: &[u8]) -> Result<Self, PxFontError> {
        let mut reader = Cursor::new(data);
        let header = FileHeader::read(&mut reader)
            .map_err(|e| PxFontError::InvalidHeader(e))?;
        let mut chars = HashMap::new();

        loop {
            let block = GlyphBlock::read(&mut reader)
                .map_err(|e| PxFontError::InvalidGlyphBlockHeader(e))?;
            let is_rgba = block.rgba > 0;
            let (start, end) = match (block.start.try_into(), block.end.try_into()) {
                (Ok(start), Ok(end)) => (start, end),
                _ => return Err(
                    PxFontError::InvalidGlyphBlockRange(block.start, block.end)
                ),
            };

            for c in start..=end {
                let mut data = remaining(&reader);
                let nr_cols = data[0];
                data = &data[1..];

                let glyph_size = match is_rgba {
                    false => nr_cols as usize * header.width as usize * header.height as usize,
                    true => nr_cols as usize * header.width as usize * header.height as usize * 4,
                };

                if data.len() < glyph_size {
                    return Err(PxFontError::InvalidGlyph(c));
                }
                let glyph = Glyph {
                    px: data[..glyph_size].to_vec(),
                    nr_cols,
                    is_rgba,
                };
                reader.seek(SeekFrom::Current(glyph_size as i64 + 1)).unwrap();
                chars.insert(c, glyph);
            }

            if remaining(&reader).is_empty() {
                break;
            }
        }

        if !chars.contains_key(&REPLACEMENT_CHARACTER) {
            return Err(PxFontError::MissingReplacementGlyph);
        }

        Ok(Self {
            chars,
            glyph_width: header.width,
            glyph_height: header.height,
        })
    }

    #[inline]
    pub fn get_glyph(&self, glyph: char) -> Option<&Glyph> {
        self.chars.get(&glyph)
    }

    #[inline]
    pub fn glyph_width(&self) -> u8 {
        self.glyph_width
    }

    #[inline]
    pub fn glyph_height(&self) -> u8 {
        self.glyph_height
    }

    #[inline]
    pub fn replacement_glyph(&self) -> &Glyph {
        &self.chars[&REPLACEMENT_CHARACTER]
    }
}

impl Glyph {
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.px
    }

    #[inline]
    pub fn is_rgba(&self) -> bool {
        self.is_rgba
    }

    #[inline]
    pub fn nr_columns(&self) -> usize {
        self.nr_cols as usize
    }
}

fn remaining<'a>(cursor: &Cursor<&'a [u8]>) -> &'a [u8] {
    &cursor.get_ref()[(cursor.position() as usize)..]
}
