export type VersionedModRepo = { [id: string]: { [version: string]: ModRepoMod } }

export interface ModRepoMod {
    name: string,
    id: string,
    version: string,
    download: string,
    source: string,
    author: string,
    cover: string | null,
    modloader: string,
    description: string
    global?: boolean
}

const repoUrlTemplate: string = "https://mods.bsquest.xyz/{0}.json"

export async function loadRepo(gameVersion: string): Promise<VersionedModRepo> {
    const req = await fetch(repoUrlTemplate.replace("{0}", gameVersion));
    return (await req.json()) as VersionedModRepo;
}