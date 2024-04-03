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

type Request = GetModStatus | Patch | SetModsEnabled;

interface Mods {
    type: 'Mods',
    installed_mods: Mod[]
}

interface ModStatus {
    type: 'ModStatus',
    app_info: AppInfo | null,
    core_mods: CoreModsInfo | null,
    modloader_present: boolean,
    installed_mods: Mod[]
}

type Response = ModStatus | Log | Mods;

interface CoreModsInfo {
    supported_versions: string[],
    all_core_mods_installed: boolean
}

interface AppInfo {
    version: string,
    is_modded: boolean
}

interface Log {
    type: 'Log',
    message: string,
    level: LogLevel
}

type LogLevel = "Error" | "Warn" | "Info" | "Debug" | "Trace";

export type {
    Request,
    GetModStatus,
    Response,
    ModStatus,
    AppInfo,
    CoreModsInfo,
    Log,
    Mods
}