//! Structures for the deserialization for the QMOD `mod.json` file.
//!
//! Taken from the QPM.qmod library at the following URL
//! https://github.com/QuestPackageManager/QPM.qmod/blob/main/src/models/mod_json.rs
//! This code is under the GNU General Public License version 3, found here:
//! https://github.com/QuestPackageManager/QPM.qmod/blob/main/LICENSE

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// The maximum number of package-visibility declarations one QMOD may request.
pub const MAX_QUERY_PACKAGES: usize = 32;

/// Returned when enabling a mod would leave one or more declared requirements unsatisfied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingManifestRequirements {
    pub mod_id: String,
    pub query_packages: Vec<String>,
}

impl Display for MissingManifestRequirements {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Mod {} requires missing manifest query packages: {}",
            self.mod_id,
            self.query_packages.join(", ")
        )
    }
}

impl std::error::Error for MissingManifestRequirements {}

/// Declarative Android manifest requirements for a QMOD.
///
/// This intentionally exposes a small, typed surface rather than accepting raw XML. New
/// requirement types should be added individually after defining their validation, compatibility,
/// and safety policy.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestRequirements {
    /// Android package IDs that must be visible through PackageManager.
    pub query_packages: Vec<String>,
}

impl ManifestRequirements {
    /// Validates limits that must remain enforced even if schema validation is bypassed or changed.
    pub fn validate(&self) -> Result<()> {
        if self.query_packages.len() > MAX_QUERY_PACKAGES {
            return Err(anyhow!(
                "A QMOD may request at most {MAX_QUERY_PACKAGES} query packages"
            ));
        }

        let mut unique_packages = HashSet::new();
        for package in &self.query_packages {
            if !is_valid_android_package_name(package) {
                return Err(anyhow!(
                    "Manifest query package `{package}` is not a valid Android package name"
                ));
            }
            if !unique_packages.insert(package) {
                return Err(anyhow!(
                    "Manifest query package `{package}` was requested more than once"
                ));
            }
        }

        Ok(())
    }

    /// Returns the requested packages that are not already declared by the app manifest.
    pub fn missing_query_packages(&self, declared: &HashSet<String>) -> Vec<String> {
        self.query_packages
            .iter()
            .filter(|package| !declared.contains(*package))
            .cloned()
            .collect()
    }
}

/// A conservative package-name validator for values written to `android:name`.
///
/// Names are limited to ASCII, contain at least two dot-separated segments, and each segment
/// starts with a letter. This covers conventional Android application IDs while excluding XML,
/// whitespace and resource syntax.
fn is_valid_android_package_name(package: &str) -> bool {
    if package.len() > 255 {
        return false;
    }

    let mut segments = package.split('.');
    let mut segment_count = 0;
    for segment in &mut segments {
        segment_count += 1;
        let mut chars = segment.chars();
        if !chars
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
            || !chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return false;
        }
    }

    segment_count >= 2
}

/// Model for the `mod.json` manifest within a QMOD.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)] // Skip missing fields
pub struct ModInfo {
    /// The Questpatcher version this mod.json was made for
    #[serde(rename(serialize = "_QPVersion", deserialize = "_QPVersion"))]
    pub schema_version: Version,
    /// Name of the mod
    pub name: String,
    /// ID of the mod
    pub id: String,
    /// Modloader. Possible values: QuestLoader/Scotland2
    pub modloader: Option<String>,
    /// Author of the mod
    pub author: String,
    /// Optional slot for if you ported a mod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub porter: Option<String>,
    /// Mod version
    pub version: Version,
    /// id of the package the mod is for, ex. com.beatgaems.beatsaber
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    /// Version of the package, ex. 1.1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
    /// description for the mod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// optional cover image filename
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    /// whether or not this qmod is a library or not
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_library: Option<bool>,
    /// list of downloadable dependencies
    pub dependencies: Vec<ModDependency>,
    /// list of files that go in the package's early mods folder
    pub mod_files: Vec<String>,
    /// list of files that go in the package's late mods folder
    pub late_mod_files: Vec<String>,
    /// list of files that go in the package's libs folder
    pub library_files: Vec<String>,
    /// list of files that will be copied on the quest
    pub file_copies: Vec<FileCopy>,
    /// list of copy extensions registered for this specific mod
    pub copy_extensions: Vec<CopyExtension>,
    /// Optional, typed requirements that must exist in the patched Android manifest before this
    /// mod is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_requirements: Option<ManifestRequirements>,
}

impl Default for ModInfo {
    fn default() -> Self {
        Self {
            schema_version: Version::new(1, 2, 0),
            name: Default::default(),
            id: Default::default(),
            author: Default::default(),
            porter: Default::default(),
            version: semver::Version::new(0, 0, 0),
            package_id: Default::default(),
            package_version: Default::default(),
            description: Default::default(),
            cover_image: Default::default(),
            is_library: Default::default(),
            dependencies: Default::default(),
            mod_files: Default::default(),
            library_files: Default::default(),
            file_copies: Default::default(),
            copy_extensions: Default::default(),
            manifest_requirements: Default::default(),
            modloader: Some("Scotland2".into()),
            late_mod_files: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModDependency {
    /// The version requirement for this dependency
    #[serde(rename = "version")]
    pub version_range: VersionReq,
    /// The id of this dependency
    pub id: String,
    /// The download link for this dependency, must satisfy id and version range!
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "downloadIfMissing")]
    pub mod_link: Option<String>,
    /// Whether this dependency must be installed for the dependant mod to be installed.
    /// If this is `false`, then the dependency must satisfy `version_range` *if* it is installed.
    #[serde(default = "true_default")]
    pub required: bool,
}

/// A QMOD file copy.
/// These can be used to copy arbitrary files to arbitrary locations on the Quest.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileCopy {
    /// name of the file in the qmod
    pub name: String,
    /// place where to put it (full path)
    pub destination: String,
}

/// A QMOD copy extension
/// Copy extensions allow mods to specify where files with particular file extensions should be copied to.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CopyExtension {
    /// the extension to register for
    pub extension: String,
    /// the destination folder these files should be going to
    pub destination: String,
}

fn true_default() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_requirements_are_optional_for_existing_qmods() {
        let manifest: ModInfo = serde_json::from_value(serde_json::json!({
            "_QPVersion": "1.2.0",
            "name": "Legacy mod",
            "id": "legacy-mod",
            "author": "Example",
            "version": "1.0.0"
        }))
        .unwrap();

        assert_eq!(manifest.manifest_requirements, None);
    }

    #[test]
    fn accepts_conventional_android_package_names() {
        let requirements = ManifestRequirements {
            query_packages: vec![
                "com.discord".to_string(),
                "com.AnotherAxiom.GorillaTag".to_string(),
                "io.example_app.client2".to_string(),
            ],
        };

        requirements.validate().unwrap();
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_package_names() {
        for invalid in [
            "discord",
            ".com.discord",
            "com..discord",
            "com.2discord",
            "com.discord-beta",
            "com.discord/other",
            "com.discord\" />",
            "com.discórd",
        ] {
            let requirements = ManifestRequirements {
                query_packages: vec![invalid.to_string()],
            };
            assert!(requirements.validate().is_err(), "accepted `{invalid}`");
        }
    }

    #[test]
    fn rejects_duplicate_or_excessive_query_packages() {
        let duplicate = ManifestRequirements {
            query_packages: vec!["com.discord".to_string(), "com.discord".to_string()],
        };
        assert!(duplicate.validate().is_err());

        let excessive = ManifestRequirements {
            query_packages: (0..=MAX_QUERY_PACKAGES)
                .map(|index| format!("com.example.package{index}"))
                .collect(),
        };
        assert!(excessive.validate().is_err());
    }

    #[test]
    fn reports_only_packages_missing_from_the_manifest() {
        let requirements = ManifestRequirements {
            query_packages: vec!["com.discord".to_string(), "com.spotify.music".to_string()],
        };
        let declared = HashSet::from(["com.discord".to_string()]);

        assert_eq!(
            requirements.missing_query_packages(&declared),
            vec!["com.spotify.music"]
        );
    }
}
