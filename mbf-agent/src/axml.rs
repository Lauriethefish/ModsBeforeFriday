//! Module to parse and save the Android binary XML format
//! Used for modifying the APK manifest

use std::{collections::HashMap, io::{Cursor, Read, Seek, SeekFrom, Write}, rc::Rc};

use anyhow::{Result, anyhow, Context};
use byteorder::{ReadBytesExt, WriteBytesExt, BE, LE};

/// An XML event within the main body of an AXML file.
#[derive(Debug, Clone)]
pub enum Event {
    /// An event that this implementation does not parse/understand, typically CData
    Unknown {
        contents: Vec<u8>,
        res_type: u32
    },
    StartNamespace(Namespace),
    EndNamespace(Namespace),
    StartElement {
        attributes: Vec<Attribute>,
        name: Rc<str>,
        namespace: Option<Rc<str>>,
        line_num: u32,
    },
    EndElement {
        line_num: u32,
        namespace: Option<Rc<str>>,
        name: Rc<str>
    }
}

#[derive(Debug, Clone)]
pub struct Namespace {
    prefix: Option<Rc<str>>,
    uri: Rc<str>
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: Rc<str>,
    pub namespace: Option<Rc<str>>,
    pub resource_id: Option<u32>,
    pub value: AttributeValue
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(Rc<str>),
    Boolean(bool),
    Integer(i32),
    Reference(u32) // Reference ID
}

pub struct AxmlReader<'r, R: Read + Seek> {
    data: &'r mut R,

    string_pool: Vec<Rc<str>>,

    // Map of resource IDs to resource map indices
    res_map: Vec<u32>,

    end_file_offset: u64
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
        let (string_pool, _was_utf8) = load_string_pool(data).context("Failed to load string pool")?;
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
            end_file_offset: file_size as u64
        })
    }

    /// Reads the next event from the file.
    pub fn read_next_event(&mut self) -> Result<Option<Event>> {
        if self.data.stream_position()? == self.end_file_offset {
            return Ok(None)
        }

        let raw_res_type = self.data.read_u32::<LE>()?;
        let length = self.data.read_u32::<LE>()?;
        let post_ev_offset = self.data.stream_position()? - 8 + length as u64;

        match ChunkType::parse(raw_res_type) {
            Some(known) => {
                let result = match known {
                    ChunkType::StringPool 
                    | ChunkType::Xml
                    | ChunkType::XmlResourceMap => return Err(anyhow!("Invalid res type {raw_res_type} for main file contents")),
                    ChunkType::XmlStartNamespace => Event::StartNamespace(self.read_namespace()?),
                    ChunkType::XmlEndNamespace => Event::EndNamespace(self.read_namespace()?),
                    ChunkType::XmlStartElement => self.read_element()?,
                    ChunkType::XmlEndElement => self.read_end_element()?,
                };

                // Make sure to seek to the start of the next element
                // (in case reading this element fails, we can continue from the next element)
                self.data.seek(SeekFrom::Start(post_ev_offset))?;
                
                Ok(Some(result))
            },
            None => {
                let mut contents = vec![0u8; length as usize];
                self.data.read_exact(&mut contents)?;

                Ok(Some(Event::Unknown { contents, res_type: raw_res_type }))
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
        }   else    {
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
           return Err(anyhow!("Element indicated presence of style, class or ID attributes, which are not supported"))
        }

        let mut attributes = Vec::with_capacity(num_attributes as usize);
        for _ in 0..num_attributes {
            attributes.push(self.read_attribute()?);
        }

        Ok(Event::StartElement {
            attributes,
            name,
            namespace,
            line_num
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
            Some(AttributeTypeId::Int) | Some(AttributeTypeId::Hex) => AttributeValue::Integer(raw_value as i32),
            Some(AttributeTypeId::String) => AttributeValue::String(self.get_pooled_string(raw_value)?),
            Some(AttributeTypeId::Reference) => AttributeValue::Reference(raw_value),
            None => return Err(anyhow!("Attribute type ID {type_id} was not recognised"))
        };

        Ok(Attribute {
            name: self.get_pooled_string(name_and_res_id)?,
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
        }   else    {
            Some(self.get_pooled_string(ns_idx as u32)?.to_owned())
        };

        let name_idx = self.data.read_u32::<LE>()?;
        let name = self.get_pooled_string(name_idx)?;
        Ok(Event::EndElement { line_num, namespace, name })
    }

    fn read_namespace(&mut self) -> Result<Namespace> {
        let _line_num = self.data.read_u32::<LE>()?;

        let _unknown = self.data.read_u32::<LE>()?;
        let prefix_id = self.data.read_u32::<LE>()?;
        let uri_id = self.data.read_u32::<LE>()?;

        let prefix = if prefix_id == 0xFFFFFFFF {
            None
        }   else    {
            Some(self.get_pooled_string(prefix_id)?.to_owned())
        };

        let uri = self.get_pooled_string(uri_id)?;
        Ok(Namespace { prefix, uri })
    }

    fn get_pooled_string(&self, id: u32) -> Result<Rc<str>> {
        match self.string_pool.get(id as usize) {
            Some(s) => Ok(s.clone()),
            None => Err(anyhow!("Invalid string index {id}"))
        }
    }
}

pub struct AxmlWriter<'w, W: Write> {
    data: &'w mut W,

    string_pool: HashMap<Rc<str>, u32>,
    linear_string_pool: Vec<Rc<str>>,

    res_map: HashMap<u32, u32>, // Key is resource ID, value is res map index
    linear_res_map: Vec<u32>,

    events: Vec<Event>,
    main_contents: Cursor<Vec<u8>>
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
            Event::StartElement { attributes, .. } => 
                self.prepare_res_map(&attributes),
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
        let str_pool_padding = (4 - str_pool_len % 4) % 4;
        let res_pool_len = self.linear_res_map.len() as u32 * 4;

        // The "XML" chunk is the parent chunk of the entire file
        let total_xml_chunk_length = str_pool_len 
            + str_pool_padding 
            + res_pool_len 
            + 16 // String pool and resource map headers
            + self.main_contents.position() as u32;
        Self::write_chunk_header(self.data, ChunkType::Xml, total_xml_chunk_length)?;

        Self::write_chunk_header(self.data, ChunkType::StringPool, str_pool_len + str_pool_padding)?;
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
                    Some(_) => {},
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
            },
            Event::StartNamespace(ns) => self.write_start_namespace(ns)?,
            Event::EndNamespace(ns) => self.write_end_namespace(ns)?,
            Event::StartElement { attributes, name, namespace, line_num } =>
                 self.write_start_element(attributes, name, namespace, line_num)?,
            Event::EndElement { line_num, namespace, name } => 
                self.write_end_element(line_num, namespace, name)?,
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
    
    fn write_start_element(&mut self, mut attributes: Vec<Attribute>,
        name: Rc<str>,
        namespace: Option<Rc<str>>,
        line_num: u32) -> Result<()> {
        Self::write_chunk_header(&mut self.main_contents,
            ChunkType::XmlStartElement,
            Self::get_start_element_len(attributes.len()
                .try_into().context("Too many attributes for element")?)
        )?;

        self.main_contents.write_u32::<LE>(line_num)?;
        self.main_contents.write_i32::<LE>(-1)?;
        if let Some(uri) = namespace {
            let uri_idx = self.get_string_idx(uri)?;
            self.main_contents.write_u32::<LE>(uri_idx)?;
        }   else {
            self.main_contents.write_i32::<LE>(-1)?;
        }

        let name_idx = self.get_string_idx(name)?;
        self.main_contents.write_u32::<LE>(name_idx)?;
        self.main_contents.write_u32::<LE>(0x00140014)?;

        self.main_contents.write_u16::<LE>(attributes.len()
            .try_into()
            .context("Too many attributes")?)?;
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

    fn write_end_element(&mut self,
        line_number: u32,
        namespace: Option<Rc<str>>,
        name: Rc<str>) -> Result<()> {
        Self::write_chunk_header(&mut self.main_contents, ChunkType::XmlEndElement, 16)?;
        self.main_contents.write_u32::<LE>(line_number)?;
        self.main_contents.write_i32::<LE>(-1)?;
        
        if let Some(uri) = namespace {
            let uri_idx = self.get_string_idx(uri)?;
            self.main_contents.write_u32::<LE>(uri_idx)?;
        }   else {
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
        }   else    {
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
        }   else {
            self.main_contents.write_i32::<LE>(-1)?;
        }
    
        let name_idx = if let Some(res_id) = attribute.resource_id {
            self.get_res_map_and_string_pool_idx(res_id, attribute.name)
        }   else    {
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
            }
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
    fn get_string_idx(&mut self, s: Rc<str>) -> Result<u32> {
        match self.string_pool.get(&s) {
            Some(idx) => Ok(*idx),
            None => {
                let new_idx = self.string_pool.len().try_into()
                    .context("String pool too large")?;

                self.string_pool.insert(s.clone(), new_idx);
                self.linear_string_pool.push(s);
                Ok(new_idx)
            },
        }
    }

    // For resource IDs, the resource ID in the resource map and the attribute name must
    // have matching indices in the string pool/resource map.
    // This function verifies that this is the case and then returns the ID to use.
    fn get_res_map_and_string_pool_idx(&mut self, res_id: u32, attr_name: Rc<str>) -> Result<u32> {
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
        1 + if str.len() > 0x7F {
            2
        }   else    {
            1
        } + str.len()
    }

    // Saves the AXML string pool, as UTF-8
    fn write_string_pool(&mut self) -> Result<()> {
        self.data.write_u32::<LE>(self.string_pool.len()
            .try_into().context("String pool length too large")?)?;
        self.data.write_u32::<LE>(0)?; // Style count, not implemented
        self.data.write_u32::<LE>(UTF8_FLAG)?; // Strings are saved as UTF-8

        // Offset from the start of the chunk to the first byte of the first string
        let strings_offset = 7 * 4 + self.string_pool.len() * 4;
        self.data.write_u32::<LE>(strings_offset.try_into().context("String pool too large")?)?;
        self.data.write_u32::<LE>(0)?; // Purpose unknown

        // Write out the offset to each string within the pool
        let mut curr_str_offset = 0; // Ignore the initial 0 byte
        for str in self.linear_string_pool.iter() {
            self.data.write_u32::<LE>(curr_str_offset.try_into().context("String pool too large")?)?;
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

const UTF8_FLAG: u32 = 0x00000100;
fn load_string_pool(data: &mut (impl Read + Seek)) -> Result<(Vec<Rc<str>>, bool)> {
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

    let mut result: Vec<Rc<str>> = Vec::with_capacity(num_strings as usize);
    for offset in string_offsets.into_iter() {
        data.seek(SeekFrom::Start(begin_chunk + string_data_offset as u64 + offset as u64))?;

        if utf8 {
            let _ = read_utf8_len(data)?; // TODO: Figure out what this represents

            // TODO: Apparently extra bytes can exist beyond the end of this length according to our previous implementation
            // Check if this is actually the case.
            let length = read_utf8_len(data)? as usize;
            let mut buffer = vec![0u8; length];
            data.read_exact(&mut buffer)?;

            result.push(std::str::from_utf8(&buffer)?.into());
        }   else {
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
    if length & 0x80 != 0 { // Last bit set, so length is 2 bytes
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

// Writes the given length as the varint used to represent the length of a UTF8 string in AXML
fn write_utf8_len(data: &mut impl Write, len: usize) -> Result<()> {
    if len > 0x7FFF {
        return Err(anyhow!("String length is too long to save as UTF-8 {}", len))
    }   else if len > 0x7F {
        data.write_u16::<BE>((len | 0x8000) as u16)?;
    }   else {
        data.write_u8(len as u8)?;
    }

    Ok(())
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum ChunkType {
    StringPool,
    Xml,
    XmlResourceMap,
    XmlStartNamespace,
    XmlEndNamespace,
    XmlStartElement,
    XmlEndElement
}

impl ChunkType {
    pub fn parse(from: u32) -> Option<Self> {
        match from & 0xFFFF {
            0x0001 => Some(Self::StringPool),
            0x0003 => Some(Self::Xml),
            0x0180 => Some(Self::XmlResourceMap),
            0x0103 => Some(Self::XmlEndElement),
            0x0100 => Some(Self::XmlStartNamespace),
            0x0101 => Some(Self::XmlEndNamespace),
            0x0102 => Some(Self::XmlStartElement),
            _ => None
        }
    }

    pub fn save(&self) -> u32 {
        let prefix = match self {
            ChunkType::StringPool => 0x001C,
            ChunkType::XmlResourceMap | ChunkType::Xml => 0x0008,
            _ => 0x0010
        };

        let id = match self {
            ChunkType::StringPool => 0x0001,
            ChunkType::Xml => 0x0003,
            ChunkType::XmlResourceMap => 0x0180,
            ChunkType::XmlStartNamespace => 0x0100,
            ChunkType::XmlEndNamespace => 0x0101,
            ChunkType::XmlStartElement => 0x0102,
            ChunkType::XmlEndElement => 0x0103,
        };

        id | prefix << 16
    }
}

#[derive(Copy, Clone)]
enum AttributeTypeId {
    Int,
    Boolean,
    Hex,
    Reference,
    String
}

impl AttributeTypeId {
    pub fn parse(from: u32) -> Option<AttributeTypeId> {
        match from >> 24 {
            0x10 => Some(Self::Int),
            0x12 => Some(Self::Boolean),
            0x11 => Some(Self::Hex),
            0x01 => Some(Self::Reference),
            0x03 => Some(Self::String),
            _ => None
        }
    }

    pub fn save(&self) -> u32 {
        let basic_type = match self {
            Self::Int => 0x10,
            Self::Boolean => 0x12,
            Self::Hex => 0x11,
            Self::Reference => 0x01,
            Self::String => 0x03
        };

        (basic_type << 24) | 0x000008
    }
}