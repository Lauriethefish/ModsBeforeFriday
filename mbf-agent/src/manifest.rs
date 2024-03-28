//! Module containing convenience functions for modifying AndroidManifest.xml

use std::{collections::HashMap, io::{Cursor, Read, Seek, Write}, rc::Rc};

use anyhow::{Context, Result, anyhow};
use byteorder::{ReadBytesExt, LE};

use crate::axml::{Attribute, AttributeValue, AxmlReader, AxmlWriter, Event};

const ANDROID_NS_URI: &str = "http://schemas.android.com/apk/res/android";
const RESOURCE_ID_TABLE: &[u8] = include_bytes!("resourceIds.bin");

pub struct ManifestInfo {
    pub package_version: String
}

impl ManifestInfo {
    pub fn read<T: Read + Seek>(reader: &mut AxmlReader<T>) -> Result<Self> {
        let mut version: Option<String> = None;
        while let Some(event) = reader.read_next_event()? {
            match event {
                Event::StartElement {
                    attributes,
                    name,
                    .. 
                } => {
                        if &*name != "manifest" {
                            continue;
                        }
    
                        let version_attr = attributes.iter()
                            .filter(|attr| &*attr.name == "versionName")
                            .next();
    
                        match version_attr {
                            Some(attr) => match &attr.value {
                                AttributeValue::String(s) => version = Some(s.to_string()),
                                _ => return Err(anyhow!("Package version must be a string"))
                            },
                            None => return Err(anyhow!("No package version attribute"))
                        }
                    },
                _ => {}
            }
        }

        match version {
            Some(package_version) => Ok(Self {
                package_version
            }),
            None => Err(anyhow!("No useful information found in the manifest"))
        }
    }
}


/// Convenient builder that can be used to modify the APK manifest
pub struct ManifestMod {
    queued_perms: Vec<Rc<str>>,
    queued_features: Vec<Rc<str>>,
    debuggable: bool
}

impl ManifestMod {
    pub fn new() -> Self {
        Self {
            queued_perms: Vec::new(),
            queued_features: Vec::new(),
            debuggable: false
        }
    }

    pub fn with_feature(mut self, feature: &str) -> Self {
        self.queued_features.push(feature.into());
        self
    }

    pub fn with_permission(mut self, feature: &str) -> Self {
        self.queued_perms.push(feature.into());
        self
    }

    pub fn debuggable(mut self, debuggable: bool) -> Self {
        self.debuggable = debuggable;
        self
    }

    // Adds the requested permissions and features to the APK
    pub fn apply_mod<R: Read + Seek, W: Write>(&self,
        reader: &mut AxmlReader<R>,
        writer: &mut AxmlWriter<W>,
        res_ids: &ResourceIds) -> Result<()> {

        while let Some(mut ev) = reader.read_next_event().context("Failed to read original manifest")? {
            let is_closing = match &mut ev { // Determine if the current event is the final tag: </manifest>
                Event::StartElement { attributes, name, .. } => {
                    // Add the debuggable attribute onto the "application" element
                    if &**name == "application" && self.debuggable {
                        attributes.push(
                            android_attribute("debuggable", AttributeValue::Boolean(true), res_ids)
                        );
                    }
                    false
                },
                // Locate the closing </manifest> tag
                Event::EndElement { name, .. } => &**name == "manifest",
                _ => false
            };

            if is_closing {
                // Write out permissions and features just before the final (closing) tag
                let uses_feature: Rc<str> = "uses-feature".into();
                let uses_permission: Rc<str> = "uses-permission".into();
                for feature in &self.queued_features {
                    write_named_element(writer, uses_feature.clone(), feature.clone(), res_ids);
                }
                for permission in &self.queued_perms {
                    write_named_element(writer, uses_permission.clone(), permission.clone(), res_ids);
                }
            }

            writer.write_event(ev);
        };

        Ok(())
    }
}


/// Stores a map of AXML attribute names to resource IDs
pub struct ResourceIds {
    ids: HashMap<Rc<str>, u32>
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

        Ok(Self {
            ids
        })
    }

    pub fn get_res_id(&self, name: &str) -> u32 {
        *self.ids.get(name).expect("No resource ID existed for given attribute name")
    }
}

fn write_named_element<W: Write>(writer: &mut AxmlWriter<W>, element_name: Rc<str>, name_value: Rc<str>, res_ids: &ResourceIds) {
    writer.write_event(Event::StartElement {
        attributes: vec![name_attribute(name_value, res_ids)],
        name: element_name.clone(),
        namespace: None,
        line_num: 0
    });
    writer.write_event(Event::EndElement {
        name: element_name,
        namespace: None,
        line_num: 0
    })
}

fn name_attribute(name: Rc<str>, res_ids: &ResourceIds) -> Attribute {
    android_attribute("name", AttributeValue::String(name), res_ids)
}

fn android_attribute(name: &str, value: AttributeValue, res_ids: &ResourceIds) -> Attribute {
    Attribute {
        name: name.into(),
        namespace: Some(ANDROID_NS_URI.into()),
        value,
        resource_id: Some(res_ids.get_res_id(name))
    }
}