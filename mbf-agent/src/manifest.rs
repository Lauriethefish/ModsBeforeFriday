//! Module containing convenience functions for modifying AndroidManifest.xml

use std::{
    collections::HashSet,
    io::{Read, Seek},
};

use anyhow::{anyhow, Result};

use mbf_axml::{AttributeValue, AxmlReader, Event, ANDROID_NS_URI};

/// Useful struct to read key details from the APK manifest.
pub struct ManifestInfo {
    pub package_version: String,
    pub query_packages: HashSet<String>,
}

impl ManifestInfo {
    pub fn read<T: Read + Seek>(reader: &mut AxmlReader<T>) -> Result<Self> {
        let mut version: Option<String> = None;
        let mut query_packages = HashSet::new();
        let mut element_depth = 0usize;
        let mut queries_depth = None;
        while let Some(event) = reader.read_next_event()? {
            match event {
                Event::StartElement {
                    attributes, name, ..
                } => {
                    element_depth += 1;
                    if name == "queries" && element_depth == 2 {
                        queries_depth = Some(element_depth);
                    }

                    if name == "manifest" {
                        let version_attr =
                            attributes.iter().find(|attr| attr.name == "versionName");

                        match version_attr {
                            Some(attr) => match &attr.value {
                                AttributeValue::String(s) => version = Some(s.to_string()),
                                _ => return Err(anyhow!("Package version must be a string")),
                            },
                            None => return Err(anyhow!("No package version attribute")),
                        }
                    } else if name == "package"
                        && queries_depth.is_some_and(|depth| element_depth == depth + 1)
                    {
                        if let Some(AttributeValue::String(package_name)) = attributes
                            .iter()
                            .find(|attribute| {
                                attribute.name == "name"
                                    && attribute.namespace.as_deref() == Some(ANDROID_NS_URI)
                            })
                            .map(|attribute| &attribute.value)
                        {
                            query_packages.insert(package_name.clone());
                        }
                    }
                }
                Event::EndElement { name, .. } => {
                    if name == "queries" && queries_depth == Some(element_depth) {
                        queries_depth = None;
                    }
                    element_depth = element_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        match version {
            Some(package_version) => Ok(Self {
                package_version,
                query_packages,
            }),
            None => Err(anyhow!("No useful information found in the manifest")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mbf_axml::AxmlWriter;

    use super::*;

    fn read_manifest(xml: &str) -> ManifestInfo {
        let mut binary_manifest = Cursor::new(Vec::new());
        let mut axml_writer = AxmlWriter::new(&mut binary_manifest);
        let mut xml_reader = xml::EventReader::new(Cursor::new(xml.as_bytes()));
        mbf_axml::xml_to_axml(&mut axml_writer, &mut xml_reader).unwrap();
        axml_writer.finish().unwrap();

        binary_manifest.set_position(0);
        let mut axml_reader = AxmlReader::new(&mut binary_manifest).unwrap();
        ManifestInfo::read(&mut axml_reader).unwrap()
    }

    #[test]
    fn reads_query_packages_from_the_manifest() {
        let info = read_manifest(
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" android:versionName="1.40.8">
                <queries>
                    <package android:name="com.discord" />
                    <package android:name="com.spotify.music" />
                </queries>
                <application />
            </manifest>"#,
        );

        assert_eq!(info.package_version, "1.40.8");
        assert_eq!(
            info.query_packages,
            HashSet::from(["com.discord".to_string(), "com.spotify.music".to_string()])
        );
    }

    #[test]
    fn ignores_package_elements_outside_direct_queries_children() {
        let info = read_manifest(
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" android:versionName="1.40.8">
                <queries><intent><package android:name="com.not-a-query-package" /></intent></queries>
                <application><package android:name="com.also-ignored" /></application>
            </manifest>"#,
        );

        assert!(info.query_packages.is_empty());
    }

    #[test]
    fn reads_all_direct_queries_and_requires_the_android_name_namespace() {
        let info = read_manifest(
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"
                          xmlns:other="https://example.com/not-android"
                          android:versionName="1.40.8">
                <queries>
                    <package other:name="com.ignored.wrongnamespace" />
                    <package android:name="com.discord" />
                </queries>
                <queries>
                    <package android:name="com.spotify.music" />
                    <package android:name="com.discord" />
                </queries>
            </manifest>"#,
        );

        assert_eq!(
            info.query_packages,
            HashSet::from(["com.discord".to_string(), "com.spotify.music".to_string()])
        );
    }
}
