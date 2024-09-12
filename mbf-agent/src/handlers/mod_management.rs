//! This file contains the request handlers relating to mod management (i.e. toggling or removing mods).
//! Adding new mods is considered to be an "importing" operation - check the [Import Handlers](crate::handlers::import)

use std::collections::HashMap;

use crate::{mod_man::ModManager, requests::{ModModel, Response}};
use anyhow::{Result, Context};
use log::info;

/// Handles `SetModsEnabled` [Requests](crate::requests::Request).
/// 
/// # Returns
/// The [Response] to the request (variant `ModSyncResult`)
pub(super) fn handle_set_mods_enabled(statuses: HashMap<String, bool>) -> Result<Response> {
    let res_cache = crate::load_res_cache()?;

    let mut mod_manager = ModManager::new(super::get_app_version_only()?, &res_cache);
    mod_manager.load_mods().context("Loading installed mods")?;

    let mut error = String::new();

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
            match mod_manager.install_mod(&id) {
                Ok(_) => info!("Installed {id}"),
                Err(err) => error.push_str(&format!("Failed to install {id}: {err}\n"))
            }
        }   else if !new_status && already_installed {
            match mod_manager.uninstall_mod(&id) {
                Ok(_) => info!("Uninstalled {id}"),
                Err(err) => error.push_str(&format!("Failed to install {id}: {err}\n"))
            }
        }
    }

    Ok(Response::ModSyncResult {
        installed_mods: get_mod_models(mod_manager)?,
        failures: if error.len() > 0 {
            if error.ends_with('\n') {
                error.pop();
            }

            Some(error)
        }   else    {
            None
        }
    })
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
        installed_mods: get_mod_models(mod_manager)?
    })
}

/// Consumes a [ModManager] and converts the loaded mods into [ModModels](ModModel) which can be serialized
/// to JSON and sent back to the frontend.
pub(super) fn get_mod_models(mut mod_manager: ModManager) -> Result<Vec<ModModel>> {
    // A dependency of one mod may have been installed in a modding operation.
    // That mod is now therefore considered installed, even though it wasn't when the mods were loaded.
    // Therefore, check dependencies of mods again to double-check which are really installed.
    mod_manager.check_mods_installed()?;

    Ok(mod_manager.get_mods()
        .map(|mod_info| ModModel::from(&*(**mod_info).borrow()))
        .collect())
}