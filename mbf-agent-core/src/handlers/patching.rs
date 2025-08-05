//! Handles requests related to the patching of the game.

use std::path::Path;

use log::{info, warn};

use crate::{mod_man::ModManager, models::response::Response, parameters::PARAMETERS, patching};
use anyhow::{anyhow, Context, Result};

/// Handles `GetDowngradedManifest` [Requests](requests::Request).
///
/// # Returns
/// The [Response](requests::Response) to the request (variant `DowngradedManifest`)
pub(super) fn handle_get_downgraded_manifest(version: String) -> Result<Response> {
    info!("Downloading manifest AXML file");
    let manifest_bytes = mbf_res_man::external_res::get_manifest_axml(
        mbf_res_man::default_agent::get_agent(),
        version,
    )
    .context("HTTP GET for downgraded AndroidManifest.xml")?;
    info!("Converting into readable XML");
    let manifest_xml = super::mod_status::axml_bytes_to_xml_string(&manifest_bytes)?;

    Ok(Response::DowngradedManifest { manifest_xml })
}

/// Handles `Patch` [Requests](requests::Request).
///
/// # Returns
/// The [Response](requests::Response) to the request (variant `Mods`)
pub(super) fn handle_patch(
    downgrade_to: Option<String>,
    repatch: bool,
    manifest_mod: String,
    device_pre_v51: bool,
    allow_no_core_mods: bool,
    override_core_mod_url: Option<String>,
    vr_splash_path: Option<String>,
) -> Result<Response> {
    let app_info =
        super::mod_status::get_app_info()?.ok_or(anyhow!("Cannot patch when app not installed"))?;
    let res_cache = crate::load_res_cache()?;

    std::fs::create_dir_all(&PARAMETERS.temp)?;

    // Either downgrade or just patch the current APK depending on the caller's choice.
    let patching_result = patching::mod_beat_saber(
        Path::new(&PARAMETERS.temp),
        &app_info,
        downgrade_to.clone(),
        manifest_mod,
        repatch,
        device_pre_v51,
        vr_splash_path.as_deref(),
        &res_cache,
    ).context("Modding the game");

    // No matter what, make sure that all temporary files are gone.
    std::fs::remove_dir_all(&PARAMETERS.temp)?;
    if let Some(splash_path) = vr_splash_path {
        std::fs::remove_file(splash_path)?;
    } // TODO add back once I fix the bugs

    let removed_dlc = patching_result?;
    patching::install_modloader().context("Installing external modloader")?;

    let new_app_version = downgrade_to.unwrap_or(app_info.version);
    let mut mod_manager = ModManager::new(new_app_version, &res_cache);

    if !repatch {
        info!("Wiping all existing mods");
        mod_manager
            .wipe_all_mods()
            .context("Wiping existing mods")?;
        mod_manager.load_mods()?; // Should load no mods.

        match super::install_core_mods(
            &res_cache,
            &mut mod_manager,
            super::mod_status::get_app_info()?
                .ok_or(anyhow!("Beat Saber should be installed after patching"))?,
            override_core_mod_url,
        ) {
            Ok(_) => info!("Successfully installed all core mods"),
            Err(err) => {
                if allow_no_core_mods {
                    warn!("Failed to install core mods: {err}")
                } else {
                    return Err(err).context("Installing core mods");
                }
            }
        }
    }

    Ok(Response::Patched {
        installed_mods: super::mod_management::get_mod_models(mod_manager)?,
        did_remove_dlc: removed_dlc,
    })
}
