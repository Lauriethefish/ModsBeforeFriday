//! AXML to XML (and vice-versa) converter
//! This allows the AXML manifest to be modified by a developer in the familiar XML format
//! and then compiled back into AXML to be used in the APK
//!
//! Important caveats: AXML contains additional data that cannot necessarily be stored in regular XML.
//! RESOURCE IDs:
//! Any AXML attribute with the android namespace URI may contain a resourceID field, which presumably provides a faster way
//! for the OS to detect what type of attribute it is? I do not truly understand why these are included.
//!
//! This project contains a map of AXML attribute names to resource IDs, used to regenerate the resource IDs from the XML file
//! at the end of conversion. IF AN ATTRIBUTE NAME IS NOT PRESENT IN THIS MAP, THEN DATA MAY BE LOST
//! This could potentially cause the manifest to be invalid. If this issue comes up, a CData event will be automatically written detailing what's missing.
//!
//! REFERENCES:
//! Certain AXML attributes contain "references" to other resources within the APK. Each reference is a 4 byte unsigned integer,
//! and they are written in the XML with the format `[REF ID_HERE]`. This is necessary as the attribute value type must be set to `reference` for Android to parse the value
//! correctly.
//!
//! DIMENSIONS:
//! Dimensions use normal text, e.g. `300.0px` or `12.5dip`. Input also accepts
//! `dp`, `sp`, `pt`, `in` and `mm`. Recompiling may normalise the packed radix.
//! `[DIM VALUE_HERE]` remains supported for raw values and unknown unit codes.
//!
//! TYPED ATTRIBUTE VALUES:
//! In regular XML, attribute values are always strings, with stringified booleans and integers etc, used for other data types.
//! In AXML attribute values can be strings, booleans, integers, references or styles (styles are not implemented currently.)
//! This allows integers/booleans to be stored more efficiently. (no string needed) but also means that, in AXML, there is a difference between
//! the string "true" and the boolean value `true`.
//!
//! When AXML attributes are converted to strings in this implementation, the values "true" "false" and any integers represent their AXML data types.

use anyhow::{ensure, Context, Result};
use std::collections::HashMap;
use xml::common::Position;

use crate::{ResourceIds, ANDROID_NS_URI};

use super::{AxmlReader, AxmlWriter};
type AxmlAttrValue = super::AttributeValue;
type AxmlEvent = super::Event;
type AxmlNamespace = super::Namespace;
type AxmlAttribute = super::Attribute;
type XmlName<'a> = xml::name::Name<'a>;

/// Converts an AXML document into readable XML format.
pub fn axml_to_xml<W: std::io::Write, R: std::io::Read + std::io::Seek>(
    writer: &mut xml::EventWriter<W>,
    reader: &mut AxmlReader<R>,
) -> Result<()> {
    use xml::writer::XmlEvent;
    let res_ids = ResourceIds::load().context("Loading resource IDs")?;

    // AXML uses a series of StartNamespace elements before a opening tag to indicate that
    // this tag declares namespaces. This Vec contains the content of any StartNamespace chunks
    // since the last opening tag.
    let mut queued_namespaces: Vec<AxmlNamespace> = Vec::new();

    // The currently available namespace prefixes. (Key is URI, value is namespace prefix)
    // Needed to correctly write the namespace prefix with each XML attribute.
    let mut current_ns_prefixes: HashMap<String, String> = HashMap::new();

    while let Some(ev) = reader.read_next_event()? {
        match ev {
            AxmlEvent::StartElement {
                attributes,
                name,
                namespace,
                line_num: _,
            } => {
                let mut string_attr_values = Vec::with_capacity(attributes.len());
                for attr in &attributes {
                    string_attr_values.push(stringify_attr_value(attr.value.clone()));
                }

                // Must be written after the element as comments cannot come before the root.
                let mut invalid_attr_errs = Vec::new();

                // Create the opening tag and add all of its attributes.
                let mut builder = XmlEvent::start_element(get_xml_name_from_axml(
                    &name,
                    &namespace,
                    &current_ns_prefixes,
                ));
                for (attr, string_value) in attributes.iter().zip(string_attr_values.iter()) {
                    builder = builder.attr(
                        get_xml_name_from_axml(&attr.name, &attr.namespace, &current_ns_prefixes),
                        &string_value,
                    );

                    if attr.namespace.as_ref().map(|ns| ns.as_str()) == Some(ANDROID_NS_URI) {
                        // Check if the resource ID for this attribute exists, and if not, write a warning before the element.
                        if let None = res_ids.get_res_id_or_none(&attr.name) {
                            invalid_attr_errs.push(format!("WARNING: Attribute `{}` has `android` namespace but no valid resource ID was found", attr.name));
                        }
                    }
                }

                // Add any queued namespaces to this attribute
                for ns in queued_namespaces.drain(0..queued_namespaces.len()) {
                    if let Some(prefix) = ns.prefix {
                        builder = builder.ns(&prefix, &ns.uri);
                    } else {
                        builder = builder.default_ns(ns.uri);
                    }
                }

                writer.write(builder)?;
                for err in invalid_attr_errs {
                    writer.write(XmlEvent::comment(&err))?;
                }

                Ok(())
            }
            AxmlEvent::EndElement {
                line_num: _,
                namespace,
                name,
            } => writer.write(
                XmlEvent::end_element()
                    // Likely not necessary but added to be on the safe side: The XML crate can automatically detect this in nearly all cases.
                    .name(get_xml_name_from_axml(
                        &name,
                        &namespace,
                        &current_ns_prefixes,
                    )),
            ),
            AxmlEvent::StartNamespace(namespace) => {
                if let Some(prefix) = namespace.prefix.clone() {
                    current_ns_prefixes.insert(namespace.uri.clone(), prefix);
                }
                queued_namespaces.push(namespace);
                Ok(())
            }
            AxmlEvent::EndNamespace(namespace) => {
                current_ns_prefixes.remove(&namespace.uri);
                Ok(())
            }
            AxmlEvent::Unknown {
                contents: _,
                res_type,
            } => writer.write(XmlEvent::comment(&format!(
                "WARNING: UNKNOWN AXML EVENT. RES_TYPE: {res_type}"
            ))),
        }?
    }

    Ok(())
}

pub fn xml_to_axml<W: std::io::Write, R: std::io::Read>(
    writer: &mut AxmlWriter<W>,
    reader: &mut xml::EventReader<R>,
) -> Result<()> {
    use xml::reader::XmlEvent;
    let res_ids = ResourceIds::load().context("Loading resource IDs")?;

    // A list of the namespaces declared by each element in the tree.
    // Will be empty if an element declares no namespaces
    // Used to determine when to write EndNamespace AXML chunks
    let mut declared_nses: Vec<Vec<AxmlNamespace>> = Vec::new();
    // Map of current namespace prefixes to namespace URIs.
    let mut current_namespace_map: HashMap<String, String> = HashMap::new();
    loop {
        match reader.next()? {
            // TODO: add support for CData in the axml parser and writer. Right now we cannot save this.
            XmlEvent::CData(_) => {}
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                // Annoyingly, we are only provided with the current map of namespaces, NOT which ones are declared with this element which is what we actually need.
                // So, we must detect which namespaces have been added with this element and write namespace declarations accordingly.
                let ns_map = namespace.0;
                let newly_declared_nses: Vec<AxmlNamespace> = ns_map
                    .iter()
                    .filter(|(prefix, uri)|
                        // These namespaces are automatically added by the rust xml crate but are implied in AXML so we can ignore them
                        **prefix != "xml" && **prefix != "xmlns" && !uri.is_empty() &&
                        current_namespace_map.get(*prefix) != Some(uri)) // Find the namespaces that have changed
                    .map(|(prefix, uri)| AxmlNamespace {
                        // Convert the String into an Option<String>, since the XML crate uses a blank string for "no prefix" in this context.
                        prefix: if prefix == xml::namespace::NS_NO_PREFIX {
                            None
                        } else {
                            Some(prefix.clone())
                        },
                        uri: uri.clone(),
                    })
                    .collect();

                // Write a StartNamespace chunk for each newly declared namespace and add it to the map
                for ns in &newly_declared_nses {
                    current_namespace_map.insert(
                        match &ns.prefix {
                            Some(prefix) => prefix.clone(),
                            None => xml::namespace::NS_NO_PREFIX.to_string(),
                        },
                        ns.uri.clone(),
                    );

                    writer.write_event(AxmlEvent::StartNamespace(ns.clone()));
                }
                // Note which namespaces have been declared with each element so we can remove them later.
                declared_nses.push(newly_declared_nses);

                let mut axml_attributes: Vec<AxmlAttribute> = Vec::new();
                for attr in attributes {
                    axml_attributes.push(AxmlAttribute {
                        // If this is an `android:` attribute then attempt to get the resource ID for it.
                        resource_id: if attr.name.namespace.as_deref() == Some(ANDROID_NS_URI) {
                            res_ids.get_res_id_or_none(&attr.name.local_name)
                        } else {
                            None
                        },
                        name: attr.name.local_name,
                        namespace: attr.name.namespace,
                        // Parse the attribute value back from its string format.
                        value: attr_value_from_string(attr.value)?,
                    })
                }

                writer.write_event(AxmlEvent::StartElement {
                    attributes: axml_attributes,
                    name: name.local_name,
                    namespace: name.namespace,
                    line_num: reader.position().row as u32,
                })
            }
            XmlEvent::EndElement { name } => {
                // Firstly end the element.
                writer.write_event(AxmlEvent::EndElement {
                    line_num: reader.position().row as u32,
                    namespace: name.namespace.map(|value| value.to_string()),
                    name: name.local_name.to_string(),
                });

                // THEN: End all of the declared namespaces in reverse order.
                let nses = declared_nses
                    .pop()
                    .expect("Must have a namespace list for each element");

                let no_prefix = xml::namespace::NS_NO_PREFIX.to_string();
                for ns in nses.iter().rev() {
                    writer.write_event(AxmlEvent::EndNamespace(ns.clone()));
                    current_namespace_map.remove(match &ns.prefix {
                        Some(prefix) => prefix,
                        None => &no_prefix,
                    });
                }
            }
            XmlEvent::EndDocument => break,
            _ => {} // No need for any other events
        }
    }

    Ok(())
}

// Converts an axml name and namespace into an XmlName struct, which wraps the name and namespace slightly differently.
fn get_xml_name_from_axml<'a>(
    name: &'a String,
    namespace: &'a Option<String>,
    // Map of namespace URIs TO namespace prefixes.
    ns_prefixes: &'a HashMap<String, String>,
) -> XmlName<'a> {
    match namespace {
        // Use the correct namespace prefix from the provided map
        Some(ns_uri) => XmlName::qualified(
            &name,
            &ns_uri,
            ns_prefixes.get(ns_uri).map(|name| name.as_str()),
        ),
        None => XmlName::local(&name),
    }
}

/// Converts an attribute value into a string to be stored as an XML value.
fn stringify_attr_value(value: AxmlAttrValue) -> String {
    match value {
        AxmlAttrValue::Boolean(b) => {
            if b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        AxmlAttrValue::Integer(i) => i.to_string(),
        AxmlAttrValue::String(s) => s,
        AxmlAttrValue::Reference(reference) => format!("[REF {reference}]"),
        AxmlAttrValue::Float(f) => f.to_string(),
        AxmlAttrValue::Dimension(value) => stringify_dimension(value),
    }
}

// Converts an attribute value back from a string to the value of an AXML attribute.
// Recognised literals are stored using their corresponding AXML attribute type.
fn attr_value_from_string(string: String) -> Result<AxmlAttrValue> {
    Ok(if string == "true" {
        AxmlAttrValue::Boolean(true)
    } else if string == "false" {
        AxmlAttrValue::Boolean(false)
    } else if let Ok(i) = string.parse::<i32>() {
        AxmlAttrValue::Integer(i)
    } else if let Ok(f) = string.parse::<f32>() {
        AxmlAttrValue::Float(f)
    } else if let Some(dimension) = string.strip_prefix("[DIM ") {
        AxmlAttrValue::Dimension(
            dimension
                .strip_suffix(']')
                .context("Invalid axml dimension: missing closing bracket")?
                .parse::<u32>()
                .context("Invalid axml dimension")?,
        )
    } else if string.starts_with("[REF ") {
        AxmlAttrValue::Reference(
            string[5..string.len() - 1]
                .parse::<u32>()
                .context("Invalid axml reference")?,
        )
    } else if let Some(dimension) = parse_dimension(&string)? {
        AxmlAttrValue::Dimension(dimension)
    } else {
        AxmlAttrValue::String(string)
    })
}

const DIMENSION_UNITS: [&str; 6] = ["px", "dip", "sp", "pt", "in", "mm"];
// Indexed by radix: 23p0, 16p7, 8p15, 0p23.
const DIMENSION_FRACTION_BITS: [u32; 4] = [0, 7, 15, 23];
const DIMENSION_MANTISSA_LIMIT: i32 = 1 << 23;

// TypedValue.complexToFloat and coerceToString:
// https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/util/TypedValue.java
fn stringify_dimension(packed: u32) -> String {
    // Bits 0-3 hold the unit, 4-5 the radix, and 8-31 the signed mantissa.
    let Some(unit) = DIMENSION_UNITS.get((packed & 0b1111) as usize) else {
        return format!("[DIM {packed}]");
    };
    let value = unpack_dimension(packed);
    // Float Debug formatting keeps a decimal point for integers (300.0px)
    // and enough digits to recover the same f32 when the XML is read again.
    format!("{value:?}{unit}")
}

fn unpack_dimension(packed: u32) -> f32 {
    let radix = ((packed >> 4) & 0b11) as usize;
    let mantissa = (packed as i32) >> 8;
    mantissa as f32 / (1u32 << DIMENSION_FRACTION_BITS[radix]) as f32
}

// None means ordinary text; an error means a numeric dimension is invalid.
fn parse_dimension(string: &str) -> Result<Option<u32>> {
    // Android's unitNames table accepts dp as an alias for dip:
    // https://android.googlesource.com/platform/frameworks/base/+/master/libs/androidfw/ResourceTypes.cpp
    let string = string.trim();
    let parsed_unit = DIMENSION_UNITS
        .iter()
        .enumerate()
        .find_map(|(unit, suffix)| {
            string.strip_suffix(suffix).map(|number| (number, unit as u32))
        })
        .or_else(|| string.strip_suffix("dp").map(|number| (number, 1)));
    let Some((number, unit)) = parsed_unit else {
        return Ok(None);
    };
    let looks_numeric = number.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '+' | '-' | '.'));
    let value = match number.parse::<f32>() {
        Ok(value) => value,
        Err(error) if looks_numeric => return Err(error).context("Invalid axml dimension number"),
        Err(_) => return Ok(None),
    };
    pack_dimension(value, unit).map(Some)
}

fn pack_dimension(value: f32, unit: u32) -> Result<u32> {
    let limit = DIMENSION_MANTISSA_LIMIT as f32;
    ensure!(
        value.is_finite() && (-limit..limit).contains(&value),
        "Invalid axml dimension: value must be finite and fit a signed 24-bit mantissa"
    );

    let radix = if value.fract() == 0.0 {
        0
    } else if value.abs() < 1.0 {
        3
    } else if value.abs() < 256.0 {
        2
    } else if value.abs() < 65536.0 {
        1
    } else {
        0
    };
    let scaled = value as f64 * (1u32 << DIMENSION_FRACTION_BITS[radix]) as f64;
    // Java Math.round rounds ties toward positive infinity. Use f64 for the
    // addition so an already integral f32 mantissa cannot round up accidentally.
    let mantissa = (scaled + 0.5).floor() as i32;
    ensure!(
        (-DIMENSION_MANTISSA_LIMIT..DIMENSION_MANTISSA_LIMIT).contains(&mantissa),
        "Invalid axml dimension: rounded value is out of range"
    );
    Ok(((mantissa as u32 & 0xffffff) << 8) | ((radix as u32) << 4) | unit)
}
