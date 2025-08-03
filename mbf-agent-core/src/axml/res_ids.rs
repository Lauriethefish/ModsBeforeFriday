use anyhow::Result;
use byteorder::{ReadBytesExt, LE};
use std::{
    collections::HashMap,
    io::{Cursor, Seek},
    rc::Rc,
};

const RESOURCE_ID_TABLE: &[u8] = include_bytes!("resourceIds.bin");

/// Stores a map of AXML attribute names to resource IDs
pub struct ResourceIds {
    ids: HashMap<Rc<str>, u32>,
}

impl ResourceIds {
    /// Loads the resource IDs from a file within the binary
    /// Ideally, reuse the same instance once you have called this method.
    pub fn load() -> Result<Self> {
        let mut file = Cursor::new(RESOURCE_ID_TABLE);
        let mut ids = HashMap::new();
        while file.stream_position()? < RESOURCE_ID_TABLE.len() as u64 {
            let name_length = file.read_u32::<LE>()? >> 1;

            let mut buffer: Vec<u16> = Vec::with_capacity(name_length as usize);
            for _ in 0..name_length {
                buffer.push(file.read_u16::<LE>()?);
            }

            let resource_id = file.read_u32::<LE>()?;
            ids.insert(String::from_utf16(&buffer)?.into(), resource_id);
        }

        Ok(Self { ids })
    }

    // Gets the resource ID for a particular attribute name. Returns None if no ID exists.
    pub fn get_res_id_or_none(&self, name: &str) -> Option<u32> {
        self.ids.get(name).map(|res_id| *res_id)
    }
}
