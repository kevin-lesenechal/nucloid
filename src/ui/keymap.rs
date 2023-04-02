use alloc::string::ToString;
use binrw::{BinRead, NullString};
use binrw::io::Cursor;
use hashbrown::HashMap;

use crate::driver::keyboard::Key;

pub struct KeymapState {
    keymap: Keymap,
    deadkey: Option<char>,
}

pub struct Keymap {
    map: HashMap<Key, CharMatrix>,
}

#[derive(Debug)]
pub enum KeymapError {
    InvalidKeymapFile,
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
