import { Mod } from "./Models";

interface GetModStatus {
    type: 'GetModStatus'
}

interface Patch {
    type: 'Patch'
}

interface SetModsEnabled {
    type: 'SetModsEnabled',
    statuses: { [id: string]: boolean } 
}

interface QuickFix {
    type: 'QuickFix'
}

interface RemoveMod {
    type: 'RemoveMod',
    id: string
}

interface Import {
    type: 'Import',
    from_path: string
}

type Request = GetModStatus | Patch | SetModsEnabled | QuickFix | RemoveMod | Import;

interface Mods {
    type: 'Mods',
    installed_mods: Mod[]
}

interface ImportedMod {
    type: 'ImportedMod',
    installed_mods: Mod[],
    imported_id: string
}

interface ImportedFileCopy {
    type: 'ImportedFileCopy',
    copied_to: string,
    mod_id: string
}

type ImportResult = ImportedMod | ImportedFileCopy;

interface ModStatus {
    type: 'ModStatus',
    app_info: AppInfo | null,
    core_mods: CoreModsInfo | null,
    modloader_present: boolean,
    installed_mods: Mod[]
}

interface LogMsg {
    type: 'LogMsg',
    message: string,
    level: LogLevel
}

type Response = LogMsg | ModStatus | Mods | ImportResult;

interface CoreModsInfo {
    supported_versions: string[],
    all_core_mods_installed: boolean
}

type ModLoader = "Scotland2" | "QuestLoader" | "Unknown";

interface AppInfo {
    version: string,
    loader_installed: ModLoader | null
}

type LogLevel = "Error" | "Warn" | "Info" | "Debug" | "Trace";

export type {
    Request,
    GetModStatus,
    Response,
    ModStatus,
    AppInfo,
    CoreModsInfo,
    LogMsg,
    Mods,
    ModLoader,
    ImportedMod,
    ImportedFileCopy,
    ImportResult
}