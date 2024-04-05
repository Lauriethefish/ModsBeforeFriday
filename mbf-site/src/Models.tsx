interface Mod {
    id: string,
    name: string,
    description: string | null,
    version: string,
    is_enabled: boolean,
    game_version: string | null
}

interface CoreMod {
    id: string,
    downloadLink: string,
    verison: string
}

interface VersionedCoreMods {
    mods: [CoreMod]
}

type CoreModIndex = { [version: string]: VersionedCoreMods }

export type {
    Mod,
    CoreMod,
    VersionedCoreMods,
    CoreModIndex
}
