import { Mod } from "./Models";

export interface GetModStatus {
    type: 'GetModStatus',
    override_core_mod_url: string | null
}

export interface Patch {
    type: 'Patch',
    manifest_mod: string,
    downgrade_to: string | null,
    allow_no_core_mods: boolean,
    override_core_mod_url: string | null,
    device_pre_v51: boolean,
    // Path to a file containing the splash image, as a PNG
    vr_splash_path: string | null,
    remodding: boolean
}

export interface FixPlayerData {
    type: 'FixPlayerData',
}

export interface SetModsEnabled {
    type: 'SetModsEnabled',
    statuses: { [id: string]: boolean } 
}

export interface QuickFix {
    type: 'QuickFix',
    override_core_mod_url: string | null,
    wipe_existing_mods: boolean
}

export interface RemoveMod {
    type: 'RemoveMod',
    id: string
}

export interface Import {
    type: 'Import',
    from_path: string
}

export interface ImportUrl {
    type: 'ImportUrl',
    from_url: string
}

export interface GetDowngradedManifest {
    type: 'GetDowngradedManifest',
    version: string
}

export type Request = GetModStatus | 
    Patch | 
    SetModsEnabled | 
    QuickFix | 
    RemoveMod | 
    Import | 
    ImportUrl | 
    FixPlayerData |
    GetDowngradedManifest;

export interface Mods {
    type: 'Mods',
    installed_mods: Mod[]
}

export interface ModSyncResult {
    type: 'ModSyncResult',
    installed_mods: Mod[],
    failures: string | null
}

export interface Patched {
    type: 'Patched',
    installed_mods: Mod[],
    did_remove_dlc: boolean
}

export interface ImportedMod {
    type: 'ImportedMod',
    installed_mods: Mod[],
    imported_id: string
}

export interface ImportedFileCopy {
    type: 'ImportedFileCopy',
    copied_to: string,
    mod_id: string
}

export interface ImportedSong {
    type: 'ImportedSong'
}

export interface NonQuestModDetected {
    type: 'NonQuestModDetected'
}

export interface FixedPlayerData {
    type: 'FixedPlayerData',
    existed: boolean
}

export interface ImportResult {
    result: ImportResultType,
    used_filename: string,
    type: 'ImportResult'
}

export type ImportResultType = ImportedMod | ImportedFileCopy | ImportedSong | NonQuestModDetected;

// Represents whether a particular part of the modded game is installed and up to date.
export type InstallStatus = "Ready" | "NeedUpdate" | "Missing";

export interface ModStatus {
    type: 'ModStatus',
    app_info: AppInfo | null,
    core_mods: CoreModsInfo | null,
    modloader_install_status: InstallStatus,
    installed_mods: Mod[],
}

export interface LogMsg {
    type: 'LogMsg',
    message: string,
    level: LogLevel
}

export interface DowngradedManifest {
    type: 'DowngradedManifest',
    manifest_xml: string
}

export type Response = LogMsg | ModStatus | Mods | ImportResult | FixedPlayerData | DowngradedManifest | Patched | ModSyncResult;

export interface CoreModsInfo {
    supported_versions: string[],
    downgrade_versions: string[],
    core_mod_install_status: InstallStatus,
    is_awaiting_diff: boolean
}

export type ModLoader = "Scotland2" | "QuestLoader" | "Unknown";

export interface AppInfo {
    version: string,
    obb_present: boolean,
    loader_installed: ModLoader | null,
    manifest_xml: string
}

export type LogLevel = "Error" | "Warn" | "Info" | "Debug" | "Trace";