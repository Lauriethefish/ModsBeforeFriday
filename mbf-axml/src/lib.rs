//! Module to parse and save the Android binary XML format
//! Used for modifying the APK manifest

mod axml2xml;
mod reader;
mod res_ids;
mod writer;

pub use reader::AxmlReader;
pub use res_ids::ResourceIds;
pub use writer::AxmlWriter;
const UTF8_FLAG: u32 = 0x00000100;
pub const ANDROID_NS_URI: &str = "http://schemas.android.com/apk/res/android";
pub use axml2xml::{axml_to_xml, xml_to_axml};

/// An XML event within the main body of an AXML file.
#[derive(Debug, Clone)]
pub enum Event {
    /// An event that this implementation does not parse/understand, typically CData
    Unknown {
        contents: Vec<u8>,
        res_type: u32,
    },
    StartNamespace(Namespace),
    EndNamespace(Namespace),
    StartElement {
        attributes: Vec<Attribute>,
        name: String,
        namespace: Option<String>,
        line_num: u32,
    },
    EndElement {
        line_num: u32,
        namespace: Option<String>,
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub prefix: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub namespace: Option<String>,
    pub resource_id: Option<u32>,
    pub value: AttributeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    Boolean(bool),
    Integer(i32),
    Reference(u32), // Reference ID
    Float(f32),
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum ChunkType {
    StringPool,
    Xml,
    XmlResourceMap,
    XmlStartNamespace,
    XmlEndNamespace,
    XmlStartElement,
    XmlEndElement,
}

impl ChunkType {
    fn parse(from: u32) -> Option<Self> {
        match from & 0xFFFF {
            0x0001 => Some(Self::StringPool),
            0x0003 => Some(Self::Xml),
            0x0180 => Some(Self::XmlResourceMap),
            0x0103 => Some(Self::XmlEndElement),
            0x0100 => Some(Self::XmlStartNamespace),
            0x0101 => Some(Self::XmlEndNamespace),
            0x0102 => Some(Self::XmlStartElement),
            _ => None,
        }
    }

    fn save(&self) -> u32 {
        let prefix = match self {
            ChunkType::StringPool => 0x001C,
            ChunkType::XmlResourceMap | ChunkType::Xml => 0x0008,
            _ => 0x0010,
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
    String,
    Float,
}

impl AttributeTypeId {
    fn parse(from: u32) -> Option<AttributeTypeId> {
        match from >> 24 {
            0x10 => Some(Self::Int),
            0x12 => Some(Self::Boolean),
            0x11 => Some(Self::Hex),
            0x01 => Some(Self::Reference),
            0x03 => Some(Self::String),
            0x04 => Some(Self::Float),
            _ => None,
        }
    }

    fn save(&self) -> u32 {
        let basic_type = match self {
            Self::Int => 0x10,
            Self::Boolean => 0x12,
            Self::Hex => 0x11,
            Self::Reference => 0x01,
            Self::String => 0x03,
            Self::Float => 0x04,
        };

        (basic_type << 24) | 0x000008
    }
}
