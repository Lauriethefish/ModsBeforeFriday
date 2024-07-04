//! Module containing convenience functions for modifying AndroidManifest.xml

use std::{collections::HashSet, io::{Read, Seek, Write}};

use anyhow::{Context, Result, anyhow};
use log::info;
use serde::Deserialize;

use crate::axml::{Attribute, AttributeValue, AxmlReader, AxmlWriter, Event, ResourceIds, ANDROID_NS_URI};

const METADATA_TAG: &str = "com.modsbeforefriday.modded";

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
#[derive(Deserialize)]
pub struct ManifestMod {
    add_permissions: Vec<String>,
    add_features: Vec<String>,
    #[serde(default = "bool::default")]
    debuggable: bool,
    #[serde(default = "bool::default")]
    hardware_accelerated: bool
}

impl ManifestMod {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            add_permissions: Vec::new(),
            add_features: Vec::new(),
            debuggable: false,
            hardware_accelerated: false,
        }
    }

    /// May be used in the future to add features, e.g. hand tracking.
    /// Currently not supported.
    #[allow(unused)]
    pub fn with_feature(mut self, feature: &str) -> Self {
        self.add_features.push(feature.into());
        self
    }

    pub fn with_permission(mut self, feature: &str) -> Self {
        self.add_permissions.push(feature.into());
        self
    }

    pub fn debuggable(mut self, debuggable: bool) -> Self {
        self.debuggable = debuggable;
        self
    }

    pub fn hardware_accelerated(mut self, hardware_accelerated: bool) -> Self {
        self.hardware_accelerated = hardware_accelerated;
        self
    }

    // Set the given attribute on the given attribute list for the <application ...> element to "true".
    // Returns true if any value was actually changed, false otherwise.
    // Will add the `android:` namespace prefix to the attribute.
    fn set_attribute_to_true(attributes: &mut Vec<Attribute>, res_ids: &ResourceIds, attr_name: &str) -> bool {
        if let Some(existing_attr) = attributes
            .iter_mut()
            .find(|attr| &*attr.name == attr_name) {
            // Set the value of the current instance of an attribute if it exists
            if existing_attr.value != AttributeValue::Boolean(true) {
                existing_attr.value = AttributeValue::Boolean(true);
                true
            }   else {
                false
            }
        }   else    {
            // Add the attribute if one doesn't already exist.
            attributes.push(
                android_attribute(attr_name, AttributeValue::Boolean(true), res_ids)
            );
            true
        }
    }

    fn get_name_attribute(attributes: &[Attribute]) -> Result<&str> {
        match &attributes.iter()
            .filter(|attr| &*attr.name == "name")
            .next()
            .ok_or(anyhow!("No valid `name` attribute existed"))?.value
        {
            AttributeValue::String(s) => Ok(s),
            _ => Err(anyhow!("`name` attribute had non-string value!"))
        }
    }

    // Adds the requested permissions and features to the APK
    // Returns true if any changes were actually made, false otherwise.
    // This can be used to avoid overwriting the manifest which would add to the ZIP size due to the naive ZIP library.
    pub fn apply_mod<R: Read + Seek, W: Write>(&self,
        reader: &mut AxmlReader<R>,
        writer: &mut AxmlWriter<W>,
        res_ids: &ResourceIds) -> Result<bool> {
        let mut modified = false;

        let mut existing_features = HashSet::new();
        let mut existing_permissions = HashSet::new();
        let mut skipping_subsequent = false;

        while let Some(mut ev) = reader.read_next_event().context("Failed to read original manifest")? {
            let is_end_of_manifest = match &mut ev { // Determine if the current event is the final tag: </manifest>
                Event::StartElement { attributes, name, .. } => {
                    if &**name == "application" {
                        if self.debuggable {
                            info!("Making debuggable");
                            modified |= Self::set_attribute_to_true(attributes, res_ids, "debuggable");
                        }
                        if self.hardware_accelerated {
                            info!("Making hardware accelerated");
                            modified |= Self::set_attribute_to_true(attributes, res_ids, "hardwareAccelerated");
                        }
                        
                    }   else if &**name == "meta-data" && Self::get_name_attribute(attributes) // Locate existing modded metadata tag
                        .is_ok_and(|name| &*name == METADATA_TAG) {
                        skipping_subsequent = true; // Skip adding permissions/feats to the manifest that were added last time we patched.
                    }   else if &**name == "uses-permission" && !skipping_subsequent {
                        // Silently fail for permissions without a name attribute
                        // TODO: figure out why some permissions/features in the Beat Saber manifest don't have one.
                        let _ = Self::get_name_attribute(attributes)
                            .map(|permission| existing_permissions.insert(permission.to_owned()));
                    }   else if &**name == "uses-feature" && !skipping_subsequent {
                        let _ = Self::get_name_attribute(attributes)
                            .map(|feature| existing_features.insert(feature.to_owned()));
                    }
                    false
                },
                // Locate the closing </manifest> tag
                Event::EndElement { name, .. } => &**name == "manifest",
                _ => false
            };

            if is_end_of_manifest {
                // Before writing out the permissions and features, write a metadata tag indicating that the app was modded by MBF.
                write_valued_element(writer,
                    "meta-data".into(),
                    METADATA_TAG.to_string().into(),
                    AttributeValue::Boolean(true),
                    res_ids
                );

                // Write out permissions and features just before the final (closing) tag
                for feature in &self.add_features {
                    if !existing_features.contains(feature.as_str()) {
                        info!("Adding feature `{feature}`");
                        write_named_element(writer, "uses-feature".to_string(), feature.clone(), res_ids);
                        modified = true;
                    }
                }
                for permission in &self.add_permissions {
                    if !existing_permissions.contains(permission.as_str()) {
                        info!("Adding permission `{permission}`");
                        write_named_element(writer, "uses-permission".to_string(), permission.clone(), res_ids);
                        modified = true;
                    }
                }
                // Make sure that the final closing tag gets written
                skipping_subsequent = false;
            }

            if !skipping_subsequent {
                writer.write_event(ev);
            }
        }

        Ok(modified)
    }
}

// Writes an element with the "name" and "value attributes"
fn write_valued_element<W: Write>(writer: &mut AxmlWriter<W>,
    element_name: String,
    name: String,
    value: AttributeValue,
    res_ids: &ResourceIds) {
    writer.write_event(Event::StartElement {
        attributes: vec![
            name_attribute(name, res_ids),
            value_attribute(value, res_ids)
        ],
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

// Writes an element with the "name" attribute.
fn write_named_element<W: Write>(writer: &mut AxmlWriter<W>, element_name: String, name_value: String, res_ids: &ResourceIds) {
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

fn name_attribute(name: String, res_ids: &ResourceIds) -> Attribute {
    android_attribute("name", AttributeValue::String(name), res_ids)
}

fn value_attribute(value: AttributeValue, res_ids: &ResourceIds) -> Attribute {
    android_attribute("value", value, res_ids)
}

fn android_attribute(name: &str, value: AttributeValue, res_ids: &ResourceIds) -> Attribute {
    Attribute {
        name: name.into(),
        namespace: Some(ANDROID_NS_URI.into()),
        value,
        resource_id: Some(res_ids.get_res_id(name))
    }
}