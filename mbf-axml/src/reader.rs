//! LICENSING
//! Although the MBF project is licensed under the GNU Affero General Public License,
//! this file and any others with the same notice (but NO FILES OTHER THAN THAT) are also available under the MIT License.
//! Copyright 2024 Laurie ?
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
//! associated documentation files (the “Software”), to deal in the Software without restriction,
//! including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
//! and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
//! INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
//! WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::{Read, Seek, SeekFrom};

use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, LE};

use super::{Attribute, AttributeTypeId, AttributeValue, ChunkType, Event, Namespace, UTF8_FLAG};

pub struct AxmlReader<'r, R: Read + Seek> {
    data: &'r mut R,

    string_pool: Vec<String>,

    // Map of resource IDs to resource map indices
    res_map: Vec<u32>,

    end_file_offset: u64,
}

impl<'r, R: Read + Seek> AxmlReader<'r, R> {
    pub fn new(data: &'r mut R) -> Result<Self> {
        // The initial structure of the AXML document is an XML tag, which contains, in order:
        // The StringPool, then the XmlResourceMap, then all of the tags within a file

        if ChunkType::parse(data.read_u32::<LE>()?) != Some(ChunkType::Xml) {
            return Err(anyhow!("Initial chunk was not XML"));
        }

        let file_size = data.read_u32::<LE>()?;
        if ChunkType::parse(data.read_u32::<LE>()?) != Some(ChunkType::StringPool) {
            return Err(anyhow!("Expected string pool after first XML tag"));
        }
        let post_string_pool = data.read_u32::<LE>()? as u64 + data.stream_position()? - 8;
        let (string_pool, _was_utf8) = load_string_pool(data).context("Loading string pool")?;
        data.seek(SeekFrom::Start(post_string_pool))?;

        let c_type = ChunkType::parse(data.read_u32::<LE>()?);
        if c_type != Some(ChunkType::XmlResourceMap) {
            return Err(anyhow!("Expected resource map after string pool"));
        }
        let res_map_len = data.read_u32::<LE>()?;
        let post_resource_map = data.stream_position()? + res_map_len as u64 - 8;

        // Number of integers within the resource map. Subtract 2 due to the chunk type and length
        let res_map_size = (res_map_len >> 2) - 2;
        let mut res_map = Vec::with_capacity(res_map_size as usize);
        for _ in 0..res_map_size {
            res_map.push(data.read_u32::<LE>()?);
        }

        data.seek(SeekFrom::Start(post_resource_map))?;

        Ok(Self {
            data,
            string_pool,
            res_map,
            end_file_offset: file_size as u64,
        })
    }

    /// Reads the next event from the file.
    pub fn read_next_event(&mut self) -> Result<Option<Event>> {
        if self.data.stream_position()? == self.end_file_offset {
            return Ok(None);
        }

        let raw_res_type = self.data.read_u32::<LE>()?;
        let length = self.data.read_u32::<LE>()?;
        let post_ev_offset = self.data.stream_position()? - 8 + length as u64;

        match ChunkType::parse(raw_res_type) {
            Some(known) => {
                let result = match known {
                    ChunkType::StringPool | ChunkType::Xml | ChunkType::XmlResourceMap => {
                        return Err(anyhow!(
                            "Invalid res type {raw_res_type} for main file contents"
                        ))
                    }
                    ChunkType::XmlStartNamespace => Event::StartNamespace(self.read_namespace()?),
                    ChunkType::XmlEndNamespace => Event::EndNamespace(self.read_namespace()?),
                    ChunkType::XmlStartElement => self.read_element()?,
                    ChunkType::XmlEndElement => self.read_end_element()?,
                };

                // Make sure to seek to the start of the next element
                // (in case reading this element fails, we can continue from the next element)
                self.data.seek(SeekFrom::Start(post_ev_offset))?;

                Ok(Some(result))
            }
            None => {
                let mut contents = vec![0u8; length as usize];
                self.data.read_exact(&mut contents)?;

                Ok(Some(Event::Unknown {
                    contents,
                    res_type: raw_res_type,
                }))
            }
        }
    }

    fn read_element(&mut self) -> Result<Event> {
        let line_num = self.data.read_u32::<LE>()?;

        if self.data.read_u32::<LE>()? != 0xFFFFFFFF {
            return Err(anyhow!("Expected -1"));
        }

        let ns_idx = self.data.read_i32::<LE>()?;
        let namespace = if ns_idx == -1 {
            None
        } else {
            Some(self.get_pooled_string(ns_idx as u32)?.to_owned())
        };

        let name_idx = self.data.read_u32::<LE>()?;
        let name = self.get_pooled_string(name_idx)?.to_owned();
        if self.data.read_u32::<LE>()? != 0x00140014 {
            return Err(anyhow!("Expected 0x00140014"));
        }

        // TODO: Can these be used in manifests? If so, we should add support for them.
        let num_attributes = self.data.read_u16::<LE>()?;
        let id_attr_idx = self.data.read_u16::<LE>()?;
        let class_attr_idx = self.data.read_u16::<LE>()?;
        let style_attr_idx = self.data.read_u16::<LE>()?;
        if id_attr_idx != 0 || class_attr_idx != 0 || style_attr_idx != 0 {
            return Err(anyhow!("Element indicated presence of style, class or ID attributes, which are not supported"));
        }

        let mut attributes = Vec::with_capacity(num_attributes as usize);
        for _ in 0..num_attributes {
            attributes.push(self.read_attribute()?);
        }

        Ok(Event::StartElement {
            attributes,
            name,
            namespace,
            line_num,
        })
    }

    fn read_attribute(&mut self) -> Result<Attribute> {
        let attr_ns = self.string_pool.get(self.data.read_u32::<LE>()? as usize);
        let name_and_res_id = self.data.read_u32::<LE>()?;

        let _raw_string_idx = self.data.read_u32::<LE>()?; // Only used for id, class and style attributes
        let type_id = self.data.read_u32::<LE>()?;
        let raw_value = self.data.read_u32::<LE>()?;

        let value = match AttributeTypeId::parse(type_id) {
            Some(AttributeTypeId::Boolean) => AttributeValue::Boolean(raw_value > 0),
            Some(AttributeTypeId::Int) | Some(AttributeTypeId::Hex) => {
                AttributeValue::Integer(raw_value as i32)
            }
            Some(AttributeTypeId::String) => {
                AttributeValue::String(self.get_pooled_string(raw_value)?.to_string())
            }
            Some(AttributeTypeId::Reference) => AttributeValue::Reference(raw_value),
            Some(AttributeTypeId::Float) => {
                AttributeValue::Float(f32::from_bits(raw_value))
            }
            None => return Err(anyhow!("Attribute type ID {type_id} was not recognised")),
        };

        Ok(Attribute {
            name: self.get_pooled_string(name_and_res_id)?.to_string(),
            namespace: attr_ns.cloned(),
            resource_id: self.res_map.get(name_and_res_id as usize).copied(),
            value,
        })
    }

    fn read_end_element(&mut self) -> Result<Event> {
        let line_num = self.data.read_u32::<LE>()?;
        if self.data.read_u32::<LE>()? != 0xFFFFFFFF {
            return Err(anyhow!("Expected -1"));
        }

        let ns_idx = self.data.read_i32::<LE>()?;
        let namespace = if ns_idx == -1 {
            None
        } else {
            Some(self.get_pooled_string(ns_idx as u32)?.to_owned())
        };

        let name_idx = self.data.read_u32::<LE>()?;
        let name = self.get_pooled_string(name_idx)?;
        Ok(Event::EndElement {
            line_num,
            namespace,
            name: name.to_string(),
        })
    }

    fn read_namespace(&mut self) -> Result<Namespace> {
        let _line_num = self.data.read_u32::<LE>()?;

        let _unknown = self.data.read_u32::<LE>()?;
        let prefix_id = self.data.read_u32::<LE>()?;
        let uri_id = self.data.read_u32::<LE>()?;

        let prefix = if prefix_id == 0xFFFFFFFF {
            None
        } else {
            Some(self.get_pooled_string(prefix_id)?.to_owned())
        };

        let uri = self.get_pooled_string(uri_id)?;
        Ok(Namespace {
            prefix,
            uri: uri.to_string(),
        })
    }

    fn get_pooled_string(&self, id: u32) -> Result<&str> {
        match self.string_pool.get(id as usize) {
            Some(s) => Ok(*&s),
            None => Err(anyhow!("Invalid string index {id}")),
        }
    }
}

fn load_string_pool(data: &mut (impl Read + Seek)) -> Result<(Vec<String>, bool)> {
    let begin_chunk = data.stream_position()? - 8; // -8 because of the chunk type/chunk length
    let num_strings = data.read_u32::<LE>()?;
    let _styles_offset = data.read_u32::<LE>()?; // Styles currently implemented

    let flags = data.read_u32::<LE>()?;
    // Default is UTF16 if the flag is not set
    let utf8 = (flags & UTF8_FLAG) != 0;

    let string_data_offset = data.read_u32::<LE>()?;
    let _style_offset_count = data.read_u32::<LE>()?;

    // Load the offsets of each string, which must be added to string_data_offset, then to the offset of the chunk beginning.
    // This calculates the actual location of the string data.
    let mut string_offsets = Vec::with_capacity(num_strings as usize);
    for _ in 0..num_strings {
        string_offsets.push(data.read_u32::<LE>()?);
    }

    let mut result: Vec<String> = Vec::with_capacity(num_strings as usize);
    for offset in string_offsets.into_iter() {
        data.seek(SeekFrom::Start(
            begin_chunk + string_data_offset as u64 + offset as u64,
        ))?;

        if utf8 {
            let _ = read_utf8_len(data)?; // TODO: Figure out what this represents

            // TODO: Apparently extra bytes can exist beyond the end of this length according to our previous implementation
            // Check if this is actually the case.
            let length = read_utf8_len(data)? as usize;
            let mut buffer = vec![0u8; length];
            data.read_exact(&mut buffer)?;

            result.push(std::str::from_utf8(&buffer)?.into());
        } else {
            // Length is in UTF-16 codepoints
            let length = read_utf16_len(data)? as usize;
            let mut buffer: Vec<u16> = Vec::with_capacity(length);
            for _ in 0..length {
                buffer.push(data.read_u16::<LE>()?);
            }

            result.push(String::from_utf16(&buffer)?.into());
        }
    }

    Ok((result, utf8))
}

// Reads the length of a UTF-8 string as encoded in AXML.
// This is a 1-2 byte varint, meaning its maximum value is 32767, as 1 bit is wasted.
fn read_utf8_len(data: &mut impl Read) -> Result<u16> {
    let mut length = data.read_u8()? as u16;
    if length & 0x80 != 0 {
        // Last bit set, so length is 2 bytes
        length = (length & 0x7F << 8) | data.read_u8()? as u16;
    }

    Ok(length)
}

// Reads the length of a UTF-16 string as encoded in AXML.
// This is a 2 or 4 byte varint.
fn read_utf16_len(data: &mut impl Read) -> Result<u32> {
    let mut length = data.read_u16::<LE>()? as u32;
    if length & 0x8000 != 0 {
        length = (length & 0x7FFF << 16) | data.read_u16::<LE>()? as u32;
    }

    Ok(length)
}
