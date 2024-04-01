interface Mod {
    id: string,
    name: string,
    description: string,
    version: string
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