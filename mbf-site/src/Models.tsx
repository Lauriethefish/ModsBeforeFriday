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

// Removes the build number, i.e. `_<big number>` suffix from the given game version.
export function trimGameVersion(gameVersion: string): string {
    return gameVersion.split("_")[0];
}