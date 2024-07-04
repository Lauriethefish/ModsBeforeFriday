//! Module containing convenience functions for modifying AndroidManifest.xml

use std::io::{Read, Seek};

use anyhow::{Result, anyhow};

use crate::axml::{AttributeValue, AxmlReader, Event};

/// Useful struct to read key details from the APK manifest.
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