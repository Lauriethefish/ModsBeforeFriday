export type ModRepo = { [version: string]: [ModRepoMod] }

export interface ModRepoMod {
    name: string,
    id: string,
    version: string,
    download: string,
    source: string,
    author: string,
    cover: string | null | undefined,
    modloader: string,
    description: string
}

const repoUrl: string = "https://raw.githubusercontent.com/ComputerElite/ComputerElite.github.io/main/tools/Beat_Saber/mods.json";

export async function loadRepo(): Promise<ModRepo> {
    const req = await fetch(repoUrl);
    return (await req.json()) as ModRepo;
}