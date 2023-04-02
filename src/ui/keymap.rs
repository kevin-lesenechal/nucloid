/******************************************************************************
 * Copyright © 2021-2023 Kévin Lesénéchal <kevin.lesenechal@gmail.com>        *
 * This file is part of the Nucloid operating system.                         *
 *                                                                            *
 * Nucloid is free software; you can redistribute it and/or modify it under   *
 * the terms of the GNU General Public License as published by the Free       *
 * Software Foundation; either version 2 of the License, or (at your option)  *
 * any later version. See LICENSE file for more information.                  *
 ******************************************************************************/

use alloc::string::ToString;
use binrw::{BinRead, NullString};
use binrw::io::Cursor;
use hashbrown::HashMap;

use crate::driver::keyboard::{Deadkey, Key};

pub struct KeymapState {
    keymap: Keymap,
    deadkey: Option<Deadkey>,
}

pub struct Keymap {
    map: HashMap<Key, CharMatrix>,
}

#[derive(Debug)]
pub enum KeymapError {
    InvalidKeymapFile,
}

impl KeymapState {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            deadkey: None,
        }
    }

    pub fn glyph(
        &mut self,
        key: Key,
        altgr: bool,
        capslock: bool,
        shift: bool,
    ) -> Option<char> {
        let c = self.keymap.glyph(key, altgr, capslock, shift)?;

        if let Some(deadkey) = <char as TryInto<Deadkey>>::try_into(c).ok() {
            if let Some(ref curr_deadkey) = self.deadkey {
                let c = if *curr_deadkey == deadkey {
                    deadkey.as_standalone()
                } else {
                    None
                };

                self.deadkey = None;
                c
            } else {
                self.deadkey = Some(deadkey);
                None
            }
        } else {
            if let Some(ref deadkey) = self.deadkey {
                let c = deadkey.apply(c);
                self.deadkey = None;
                c
            } else {
                Some(c)
            }
        }
    }
}

impl Keymap {
    pub fn from_file(data: &[u8]) -> Result<Self, KeymapError> {
        let mut reader = Cursor::new(data);
        let header = FileHeader::read(&mut reader)
            .map_err(|_| KeymapError::InvalidKeymapFile)?;

        let mut map = HashMap::new();

        for _ in 0..header.nr_mapping {
            let mapping = KeyMapping::read(&mut reader)
                .map_err(|_| KeymapError::InvalidKeymapFile)?;
            let key: Key = mapping.key_name.to_string().parse()
                .map_err(|_| KeymapError::InvalidKeymapFile)?;
            map.insert(key, CharMatrix([
                mapping.matrix[0].try_into().ok(),
                mapping.matrix[1].try_into().ok(),
                mapping.matrix[2].try_into().ok(),
                mapping.matrix[3].try_into().ok(),
                mapping.matrix[4].try_into().ok(),
                mapping.matrix[5].try_into().ok(),
                mapping.matrix[6].try_into().ok(),
                mapping.matrix[7].try_into().ok(),
            ]));
        }

        Ok(Self {
            map,
        })
    }

    pub fn glyph(
        &self,
        key: Key,
        altgr: bool,
        capslock: bool,
        shift: bool,
    ) -> Option<char> {
        self.map.get(&key)
            .and_then(|matrix| matrix.get(altgr, capslock, shift))
    }
}

struct CharMatrix([Option<char>; 8]);

#[derive(BinRead, Debug)]
#[br(little, magic = b"KEYMAP")]
struct FileHeader {
    nr_mapping: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
struct KeyMapping {
    key_name: NullString,
    matrix: [u32; 8],
}

impl CharMatrix {
    #[inline]
    pub fn get(
        &self,
        altgr: bool,
        capslock: bool,
        shift: bool,
    ) -> Option<char> {
        let index =
            if shift { 1 } else { 0 } << 0
                | if capslock { 1 } else { 0 } << 1
                | if altgr { 1 } else { 0 } << 2;
        self.0[index]
    }
}
