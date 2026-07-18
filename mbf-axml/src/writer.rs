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

use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

use byteorder::{WriteBytesExt, BE, LE};

use super::{Attribute, AttributeTypeId, AttributeValue, ChunkType, Event, Namespace, UTF8_FLAG};

pub struct AxmlWriter<'w, W: Write> {
    data: &'w mut W,

    string_pool: HashMap<String, u32>,
    linear_string_pool: Vec<String>,

    res_map: HashMap<u32, u32>, // Key is resource ID, value is res map index
    linear_res_map: Vec<u32>,

    events: Vec<Event>,
    main_contents: Cursor<Vec<u8>>,
}

impl<'w, W: Write> AxmlWriter<'w, W> {
    pub fn new(data: &'w mut W) -> Self {
        Self {
            data,
            string_pool: HashMap::new(),
            linear_string_pool: Vec::new(),
            res_map: HashMap::new(),
            linear_res_map: Vec::new(),
            main_contents: Cursor::new(Vec::new()),
            events: Vec::new(),
        }
    }

    pub fn write_event(&mut self, event: Event) {
        match &event {
            Event::StartElement { attributes, .. } => self.prepare_res_map(&attributes),
            _ => {}
        };

        self.events.push(event);
    }

    pub fn finish(mut self) -> Result<()> {
        // Write the contained events into the main chunk
        // These are saved to a buffer as we need to know the total file length before we can write the start of the file
        // (yes, this could be manually calculated as the elements are passed in,
        // we would also have to add all elements to the string pool at the same time, however)
        //
        // It's also not possible to carry out this process as the events are written, since we need attribute names and resource IDs to match
        // (see prepare_res_map)

        let mut events = Vec::new();
        std::mem::swap(&mut self.events, &mut events);
        for event in events {
            self.write_event_internal(event)?;
        }

        // The string pool must be padded to a multiple of 4 bytes
        let str_pool_len = self.get_total_str_pool_len() as u32;
        let str_pool_padding = 4 - str_pool_len % 4;
        let res_pool_len = self.linear_res_map.len() as u32 * 4;

        // The "XML" chunk is the parent chunk of the entire file
        let total_xml_chunk_length = str_pool_len
            + str_pool_padding
            + res_pool_len
            + 16 // String pool and resource map headers
            + self.main_contents.position() as u32;
        Self::write_chunk_header(self.data, ChunkType::Xml, total_xml_chunk_length)?;

        Self::write_chunk_header(
            self.data,
            ChunkType::StringPool,
            str_pool_len + str_pool_padding,
        )?;
        self.write_string_pool()?;
        for _ in 0..str_pool_padding {
            self.data.write_u8(0)?;
        }

        Self::write_chunk_header(self.data, ChunkType::XmlResourceMap, res_pool_len)?;
        for res_id in self.linear_res_map {
            self.data.write_u32::<LE>(res_id)?;
        }

        self.data.write_all(self.main_contents.get_mut())?;
        Ok(())
    }

    fn prepare_res_map(&mut self, attributes: &[Attribute]) {
        for attribute in attributes {
            // For all attributes with resource IDs, add these and the respective attribute names to the
            // res map/string pool NOW so that the resource pool index matches with the string pool index for each.
            // These must match as one field in the attribute corresponds to both indices.
            if let Some(res_id) = attribute.resource_id {
                match self.res_map.get(&res_id) {
                    Some(_) => {}
                    None => {
                        let res_map_idx = self.res_map.len() as u32;
                        self.res_map.insert(res_id, res_map_idx);
                        self.string_pool.insert(attribute.name.clone(), res_map_idx);
                        self.linear_res_map.push(res_id);
                        self.linear_string_pool.push(attribute.name.clone());
                    }
                }
            }
        }
    }

    fn write_event_internal(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Unknown { contents, res_type } => {
                self.main_contents.write_u32::<LE>(res_type)?;
                self.main_contents.write_u32::<LE>(contents.len() as u32)?;
                self.main_contents.write_all(&contents)?;
            }
            Event::StartNamespace(ns) => self.write_start_namespace(ns)?,
            Event::EndNamespace(ns) => self.write_end_namespace(ns)?,
            Event::StartElement {
                attributes,
                name,
                namespace,
                line_num,
            } => self.write_start_element(attributes, name, namespace, line_num)?,
            Event::EndElement {
                line_num,
                namespace,
                name,
            } => self.write_end_element(line_num, namespace, name)?,
        }

        Ok(())
    }

    fn write_start_namespace(&mut self, ns: Namespace) -> Result<()> {
        Self::write_chunk_header(&mut self.main_contents, ChunkType::XmlStartNamespace, 16)?;
        self.write_ns_chunk_contents(ns)?;
        Ok(())
    }

    fn write_end_namespace(&mut self, ns: Namespace) -> Result<()> {
        Self::write_chunk_header(&mut self.main_contents, ChunkType::XmlEndNamespace, 16)?;
        self.write_ns_chunk_contents(ns)?;
        Ok(())
    }

    // Gets the length of an XmlStartElement chunk, not including the header
    fn get_start_element_len(num_attributes: u32) -> u32 {
        28 + 20 * num_attributes
    }

    fn write_start_element(
        &mut self,
        mut attributes: Vec<Attribute>,
        name: String,
        namespace: Option<String>,
        line_num: u32,
    ) -> Result<()> {
        Self::write_chunk_header(
            &mut self.main_contents,
            ChunkType::XmlStartElement,
            Self::get_start_element_len(
                attributes
                    .len()
                    .try_into()
                    .context("Too many attributes for element")?,
            ),
        )?;

        self.main_contents.write_u32::<LE>(line_num)?;
        self.main_contents.write_i32::<LE>(-1)?;
        if let Some(uri) = namespace {
            let uri_idx = self.get_string_idx(uri)?;
            self.main_contents.write_u32::<LE>(uri_idx)?;
        } else {
            self.main_contents.write_i32::<LE>(-1)?;
        }

        let name_idx = self.get_string_idx(name)?;
        self.main_contents.write_u32::<LE>(name_idx)?;
        self.main_contents.write_u32::<LE>(0x00140014)?;

        self.main_contents
            .write_u16::<LE>(attributes.len().try_into().context("Too many attributes")?)?;
        self.main_contents.write_u16::<LE>(0)?;
        self.main_contents.write_u16::<LE>(0)?;
        self.main_contents.write_u16::<LE>(0)?;

        // Attributes must be sorted in the order of increasing resource ID
        // Otherwise, certain attributes are not properly detected by android
        attributes.sort_by_key(|attr| attr.resource_id);

        for attribute in attributes {
            self.write_attribute(attribute)?;
        }

        Ok(())
    }

    fn write_end_element(
        &mut self,
        line_number: u32,
        namespace: Option<String>,
        name: String,
    ) -> Result<()> {
        Self::write_chunk_header(&mut self.main_contents, ChunkType::XmlEndElement, 16)?;
        self.main_contents.write_u32::<LE>(line_number)?;
        self.main_contents.write_i32::<LE>(-1)?;

        if let Some(uri) = namespace {
            let uri_idx = self.get_string_idx(uri)?;
            self.main_contents.write_u32::<LE>(uri_idx)?;
        } else {
            self.main_contents.write_i32::<LE>(-1)?;
        }

        let name_idx = self.get_string_idx(name)?;
        self.main_contents.write_u32::<LE>(name_idx)?;

        Ok(())
    }

    fn write_ns_chunk_contents(&mut self, ns: Namespace) -> Result<()> {
        self.main_contents.write_i32::<LE>(-1)?;
        self.main_contents.write_i32::<LE>(-1)?;

        let prefix_idx = if let Some(prefix) = ns.prefix {
            self.get_string_idx(prefix)?
        } else {
            0xFFFFFFFF
        };

        let uri_idx = self.get_string_idx(ns.uri)?;

        self.main_contents.write_u32::<LE>(prefix_idx)?;
        self.main_contents.write_u32::<LE>(uri_idx)?;
        Ok(())
    }

    fn write_attribute(&mut self, attribute: Attribute) -> Result<()> {
        if let Some(namespace) = attribute.namespace {
            let ns_idx = self.get_string_idx(namespace)?;
            self.main_contents.write_u32::<LE>(ns_idx)?;
        } else {
            self.main_contents.write_i32::<LE>(-1)?;
        }

        let name_idx = if let Some(res_id) = attribute.resource_id {
            self.get_res_map_and_string_pool_idx(res_id, attribute.name)
        } else {
            self.get_string_idx(attribute.name)
        }?;
        self.main_contents.write_u32::<LE>(name_idx)?;

        // The purpose of the raw_str field seems to be unknown - it is -1,
        // except for strings where it takes the same value as raw_value
        let (raw_value, raw_str, value_type) = match attribute.value {
            AttributeValue::Boolean(true) => (-1, -1, AttributeTypeId::Boolean),
            AttributeValue::Boolean(false) => (0, -1, AttributeTypeId::Boolean),
            AttributeValue::Integer(i) => (i, -1, AttributeTypeId::Int),
            AttributeValue::Reference(link) => (link as i32, -1, AttributeTypeId::Reference),
            AttributeValue::String(str_value) => {
                let str_idx = self.get_string_idx(str_value)?;
                (str_idx as i32, str_idx as i32, AttributeTypeId::String)
            },
            AttributeValue::Float(f) => (f.to_bits() as i32, -1, AttributeTypeId::Float),
        };

        self.main_contents.write_i32::<LE>(raw_str)?;
        self.main_contents.write_u32::<LE>(value_type.save())?;
        self.main_contents.write_i32::<LE>(raw_value)?;
        Ok(())
    }

    // Writes the header for a chunk.
    // The `length` does not include the chunk header (8 bytes).
    fn write_chunk_header(to: &mut impl Write, chunk_type: ChunkType, length: u32) -> Result<()> {
        to.write_u32::<LE>(chunk_type.save())?;
        to.write_u32::<LE>(length + 8)?; // Account for the chunk length and chunk type, each of which is 4 bytes.

        Ok(())
    }

    // Gets the index of a string within the string pool.
    fn get_string_idx(&mut self, s: String) -> Result<u32> {
        match self.string_pool.get(&s) {
            Some(idx) => Ok(*idx),
            None => {
                let new_idx = self
                    .string_pool
                    .len()
                    .try_into()
                    .context("String pool too large")?;

                self.string_pool.insert(s.clone(), new_idx);
                self.linear_string_pool.push(s);
                Ok(new_idx)
            }
        }
    }

    // For resource IDs, the resource ID in the resource map and the attribute name must
    // have matching indices in the string pool/resource map.
    // This function verifies that this is the case and then returns the ID to use.
    fn get_res_map_and_string_pool_idx(&mut self, res_id: u32, attr_name: String) -> Result<u32> {
        match (self.res_map.get(&res_id), self.string_pool.get(&attr_name)) {
            (Some(res_idx), Some(string_idx)) => if *res_idx == *string_idx {
                Ok(*res_idx)
            }   else    {
                panic!("Resource pool index did not match string pool index for attribute {attr_name}.
                    Resource pool indices were not prepared before save phase")
            },
            (None, None) => {
                let new_idx = self.string_pool.len().try_into()
                    .context("Resource pool too large")?;

                self.res_map.insert(res_id, new_idx);
                Ok(new_idx)
            },
            _ => Err(anyhow!("Attribute with name {attr_name} and ID {res_id} does not match previous usage of {attr_name} and {res_id}.
                    Resource IDs must correspond one-to-one with attribute names"))
        }
    }

    // Gets the total length of the string pool chunk, not including the chunk header/chunk length bytes
    fn get_total_str_pool_len(&self) -> usize {
        let mut strings_len = 0;
        for str in self.string_pool.keys() {
            strings_len += self.get_pooled_str_len(str);
        }

        20 + self.linear_string_pool.len() * 4 + strings_len
    }

    // Calculates the length of the given string within the string pool
    fn get_pooled_str_len(&self, str: &str) -> usize {
        // Each string is prefixed with an extra 0 byte.
        // The purpose of this byte is unknown, I have not found an implementation that uses it
        1 + if str.len() > 0x7F { 2 } else { 1 } + str.len()
    }

    // Saves the AXML string pool, as UTF-8
    fn write_string_pool(&mut self) -> Result<()> {
        self.data.write_u32::<LE>(
            self.string_pool
                .len()
                .try_into()
                .context("String pool length too large")?,
        )?;
        self.data.write_u32::<LE>(0)?; // Style count, not implemented
        self.data.write_u32::<LE>(UTF8_FLAG)?; // Strings are saved as UTF-8

        // Offset from the start of the chunk to the first byte of the first string
        let strings_offset = 7 * 4 + self.string_pool.len() * 4;
        self.data
            .write_u32::<LE>(strings_offset.try_into().context("String pool too large")?)?;
        self.data.write_u32::<LE>(0)?; // Purpose unknown

        // Write out the offset to each string within the pool
        let mut curr_str_offset = 0; // Ignore the initial 0 byte
        for str in self.linear_string_pool.iter() {
            self.data.write_u32::<LE>(
                curr_str_offset
                    .try_into()
                    .context("String pool too large")?,
            )?;
            curr_str_offset += self.get_pooled_str_len(str);
        }

        // Now write each string within the pool
        for str in self.linear_string_pool.iter() {
            self.data.write_u8(0)?; // TODO: Figure out what this byte is for
            write_utf8_len(self.data, str.len())?;
            self.data.write_all(str.as_bytes())?;
        }

        Ok(())
    }
}

// Writes the given length as the varint used to represent the length of a UTF8 string in AXML
fn write_utf8_len(data: &mut impl Write, len: usize) -> Result<()> {
    if len > 0x7FFF {
        return Err(anyhow!(
            "String length is too long to save as UTF-8 {}",
            len
        ));
    } else if len > 0x7F {
        data.write_u16::<BE>((len | 0x8000) as u16)?;
    } else {
        data.write_u8(len as u8)?;
    }

    Ok(())
}
