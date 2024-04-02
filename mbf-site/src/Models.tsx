interface Mod {
    id: string,
    name: string,
    description: string | null,
    version: string,
    is_enabled: boolean
}

export type {
    Mod
}

export interface CoreMod {
    id: string,
    downloadLink: string,
    verison: string
}

export interface VersionedCoreMods {
    mods: [CoreMod]
}

export type CoreModIndex = { [version: string]: VersionedCoreMods }