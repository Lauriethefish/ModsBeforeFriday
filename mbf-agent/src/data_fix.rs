use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::Path};
use anyhow::{Context, Result, anyhow};

// Fixes issues with player colour schemes from 1.28 loading incorrectly on v1.35.0 or newer.
pub fn fix_colour_schemes(path: impl AsRef<Path>) -> Result<()> {
    let mut data_file_buf = Vec::with_capacity(8192);
    File::open(&path)
        .context("Failed to open player data file for reading")?
        .read_to_end(&mut data_file_buf)?;

    let mut player_data: serde_json::Value = serde_json::from_slice(&data_file_buf).context("Player data was invalid JSON")?;

    let local_players = player_data.get_mut("localPlayers")
        .ok_or(anyhow!("No localPlayers array found"))?
        .as_array_mut()
        .ok_or(anyhow!("localPlayers was not a valid array"))?;

    for player in local_players {
        let color_schemes_settings = player.get_mut("colorSchemesSettings")
            .ok_or(anyhow!("No colorSchemesSettings found"))?
            .as_object_mut()
            .ok_or(anyhow!("colorSchemesSettings were invalid"))?;

        color_schemes_settings.insert("selectedColorSchemeId".to_string(), "User0".into());
    }


    let output_str = serde_json::to_string(&player_data)
        .context("Failed to write player data")?;

    let mut writer = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path).context("Failed to open player data file for writing")?;
    writer.write_all(output_str.as_bytes())?;

    Ok(())
}