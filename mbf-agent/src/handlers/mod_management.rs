//! This file contains the request handlers relating to mod management (i.e. toggling or removing mods).
//! Adding new mods is considered to be an "importing" operation - check the [Import Handlers](crate::handlers::import)

use std::collections::HashMap;

use crate::{
    mod_man::{MissingManifestRequirements, ModManager},
    models::response::{MissingManifestRequirementModel, ModModel, Response},
};
use anyhow::{anyhow, Context, Result};
use log::info;

/// Handles `SetModsEnabled` [Requests](crate::requests::Request).
///
/// # Returns
/// The [Response] to the request (variant `ModSyncResult`)
pub(super) fn handle_set_mods_enabled(statuses: HashMap<String, bool>) -> Result<Response> {
    let app_info = super::mod_status::get_app_info()?
        .ok_or_else(|| anyhow!("Cannot enable mods when Beat Saber is not installed"))?;
    let res_cache = crate::load_res_cache()?;

    let mut mod_manager = ModManager::new(app_info.version, &res_cache);
    mod_manager.load_mods().context("Loading installed mods")?;

    let mut error = String::new();
    let mut missing_requirements = Vec::new();

    // This preflight is deliberately performed before any requested status is changed. It covers
    // every dependency that is already downloaded, allowing the UI to complete one automatic
    // repatch before mod files are copied.
    for (id, new_status) in &statuses {
        if !new_status {
            continue;
        }

        let mod_rc = match mod_manager.get_mod(id) {
            Some(mod_rc) => mod_rc,
            None => continue,
        };
        if !mod_rc.borrow().installed() {
            missing_requirements
                .extend(mod_manager.missing_manifest_requirements(id, &app_info.query_packages)?);
        }
    }

    if !missing_requirements.is_empty() {
        return Ok(Response::ModSyncResult {
            installed_mods: get_mod_models(mod_manager)?,
            failures: None,
            manifest_changes_required: requirement_models(missing_requirements),
        });
    }

    for (id, new_status) in statuses {
        let mod_rc = match mod_manager.get_mod(&id) {
            Some(m) => m,
            None => {
                error.push_str(&format!("Mod with ID {id} did not exist\n"));
                continue;
            }
        };

        let already_installed = mod_rc.borrow().installed();
        if new_status && !already_installed {
            match mod_manager.install_mod_with_manifest(&id, &app_info.query_packages) {
                Ok(_) => info!("Installed {id}"),
                Err(err) => {
                    if let Some(requirements) = err.downcast_ref::<MissingManifestRequirements>() {
                        missing_requirements.push(requirements.clone());
                    } else {
                        error.push_str(&format!("Failed to install {id}: {err}\n"));
                    }
                }
            }
        } else if !new_status && already_installed {
            match mod_manager.uninstall_mod(&id) {
                Ok(_) => info!("Uninstalled {id}"),
                Err(err) => error.push_str(&format!("Failed to install {id}: {err}\n")),
            }
        }
    }

    Ok(Response::ModSyncResult {
        installed_mods: get_mod_models(mod_manager)?,
        failures: if !error.is_empty() {
            if error.ends_with('\n') {
                error.pop();
            }

            Some(error)
        } else {
            None
        },
        manifest_changes_required: requirement_models(missing_requirements),
    })
}

fn requirement_models(
    missing_requirements: Vec<MissingManifestRequirements>,
) -> Vec<MissingManifestRequirementModel> {
    let mut models = Vec::<MissingManifestRequirementModel>::new();
    for requirement in missing_requirements {
        if let Some(existing) = models
            .iter_mut()
            .find(|existing| existing.mod_id == requirement.mod_id)
        {
            for package in requirement.query_packages {
                if !existing.query_packages.contains(&package) {
                    existing.query_packages.push(package);
                }
            }
        } else {
            models.push(MissingManifestRequirementModel {
                mod_id: requirement.mod_id,
                query_packages: requirement.query_packages,
            });
        }
    }

    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_models_merge_duplicate_reports_for_one_mod() {
        let models = requirement_models(vec![
            MissingManifestRequirements {
                mod_id: "example-mod".to_string(),
                query_packages: vec!["com.discord".to_string()],
            },
            MissingManifestRequirements {
                mod_id: "example-mod".to_string(),
                query_packages: vec!["com.discord".to_string(), "com.spotify.music".to_string()],
            },
        ]);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].mod_id, "example-mod");
        assert_eq!(
            models[0].query_packages,
            vec!["com.discord", "com.spotify.music"]
        );
    }

    #[test]
    fn requirement_models_keep_requesting_mods_separate() {
        let models = requirement_models(vec![
            MissingManifestRequirements {
                mod_id: "first-mod".to_string(),
                query_packages: vec!["com.discord".to_string()],
            },
            MissingManifestRequirements {
                mod_id: "second-mod".to_string(),
                query_packages: vec!["com.discord".to_string()],
            },
        ]);

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].mod_id, "first-mod");
        assert_eq!(models[1].mod_id, "second-mod");
    }
}

/// Handles `RemoveMod` [Requests](crate::requests::Request).
///
/// # Returns
/// The [Response] to the request (variant `Mods`)
pub(super) fn handle_remove_mod(id: String) -> Result<Response> {
    let res_cache = crate::load_res_cache()?;
    let mut mod_manager = ModManager::new(super::get_app_version_only()?, &res_cache);
    mod_manager.load_mods()?;
    mod_manager.remove_mod(&id)?;

    Ok(Response::Mods {
        installed_mods: get_mod_models(mod_manager)?,
    })
}

/// Consumes a [ModManager] and converts the loaded mods into [ModModels](ModModel) which can be serialized
/// to JSON and sent back to the frontend.
pub(super) fn get_mod_models(mut mod_manager: ModManager) -> Result<Vec<ModModel>> {
    // A dependency of one mod may have been installed in a modding operation.
    // That mod is now therefore considered installed, even though it wasn't when the mods were loaded.
    // Therefore, check dependencies of mods again to double-check which are really installed.
    mod_manager.check_mods_installed()?;

    Ok(mod_manager
        .get_mods()
        .map(|mod_info| ModModel::from(&*(**mod_info).borrow()))
        .collect())
}
