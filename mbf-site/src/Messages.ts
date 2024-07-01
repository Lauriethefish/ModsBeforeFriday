import { ManifestMod, Mod } from "./Models";

export interface GetModStatus {
    type: 'GetModStatus',
    override_core_mod_url: string | null
}

export interface Patch {
    type: 'Patch',
    manifest_mod: ManifestMod,
    downgrade_to: string | null,
    allow_no_core_mods: boolean,
    override_core_mod_url: string | null,
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

export type Request = GetModStatus | 
    Patch | 
    SetModsEnabled | 
    QuickFix | 
    RemoveMod | 
    Import | 
    ImportUrl | 
    FixPlayerData;

export interface Mods {
    type: 'Mods',
    installed_mods: Mod[]
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

export interface FixedPlayerData {
    type: 'FixedPlayerData',
    existed: boolean
}

export interface ImportResult {
    result: ImportResultType,
    used_filename: string,
    type: 'ImportResult'
}

export type ImportResultType = ImportedMod | ImportedFileCopy | ImportedSong;

export interface ModStatus {
    type: 'ModStatus',
    app_info: AppInfo | null,
    core_mods: CoreModsInfo | null,
    modloader_present: boolean,
    installed_mods: Mod[]
}

export interface LogMsg {
    type: 'LogMsg',
    message: string,
    level: LogLevel
}

export type Response = LogMsg | ModStatus | Mods | ImportResult | FixedPlayerData;

export interface CoreModsInfo {
    supported_versions: string[],
    downgrade_versions: string[],
    all_core_mods_installed: boolean,
}

export type ModLoader = "Scotland2" | "QuestLoader" | "Unknown";

export interface AppInfo {
    version: string,
    loader_installed: ModLoader | null
}

export type LogLevel = "Error" | "Warn" | "Info" | "Debug" | "Trace";