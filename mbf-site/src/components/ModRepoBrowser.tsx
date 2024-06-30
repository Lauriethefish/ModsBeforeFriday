import { useEffect, useState } from "react";
import { ModRepo, ModRepoMod, loadRepo } from "../ModsRepo";
import { ModRepoCard } from "./ModRepoCard";
import { gt as semverGt } from "semver";
import { Mod } from "../Models";

interface ModRepoBrowserProps {
    gameVersion: string,
    onDownload: (url: string) => void,
    existingMods: Mod[]
}

export function ModRepoBrowser(props: ModRepoBrowserProps) {
    const { gameVersion, onDownload } = props;
    const [modRepo, setModRepo] = useState(null as ModRepo | null);
    const [failedToLoad, setFailedToLoad] = useState(false);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        console.log("Loading mods");
        loadRepo()
            .then(repo => setModRepo(repo))
            .catch(_ => setFailedToLoad(true))
    }, []);

    if(modRepo === null) {
        if(failedToLoad) {
            return <div className="container">
                <h1>Failed to load mods</h1>
                <p>Please check that your internet is working.</p>
                <button onClick={() => {
                    setAttempt(attempt + 1);
                    setFailedToLoad(false);
                }}>Try again</button>
            </div>
        }   else    {
            return <h1>Loading mods...</h1>
        }
    }
    else if(gameVersion in modRepo) {
        return <>
            <div className="mod-list fadeIn">
                {prepareModRepoForDisplay(latestVersions(modRepo[gameVersion]), props.existingMods).map(displayInfo => 
                    <ModRepoCard
                            mod={displayInfo.mod}
                            key={displayInfo.mod.id}
                            update={displayInfo.needUpdate}
                            onInstall={() => onDownload(displayInfo.mod.download)} />
                )}
            </div>
        </>
    }   else    {
        return <>
            <h1>Failed to load mods</h1>
            <p>No mods are available for v{props.gameVersion} of the game!</p>
        </>
    }
}

interface ModDisplayInfo {
    mod: ModRepoMod,
    alreadyInstalled: boolean
    needUpdate: boolean
}

function prepareModRepoForDisplay(mods: ModRepoMod[],
    existingMods: Mod[]): ModDisplayInfo[] {
    
    return mods.map(mod => {
        // Match mods up with the existing loaded mods.
        const existingInstall = existingMods.find(existing => existing.id === mod.id);

        return {
            alreadyInstalled: existingInstall !== undefined,
            needUpdate: existingInstall !== undefined && existingInstall.version !== mod.version,
            mod: mod
        };
    }).filter(mod => mod.needUpdate || !mod.alreadyInstalled) // Skip any mods that are already installed and up to date
    .sort((a, b) => {
        // Show mods that need an update first in the list
        if(!a.needUpdate && b.needUpdate) {
            return 1;
        }

        if(!b.needUpdate && a.needUpdate) {
            return -1;
        }

        // Sort the rest of the mods alphabetically
        if(a.mod.name > b.mod.name) {
            return 1;
        }   else if(a.mod.name < b.mod.name) {
            return -1;
        }   else    {
            return 0;
        }
    });
}

// Makes the `mods` array unique by the mod `id`, extracting only the latest version of each mod.
function latestVersions(mods: ModRepoMod[]): ModRepoMod[] {
    const modsById: { [id: string]: ModRepoMod } = {};
    mods.forEach(mod => {
        if(mod.id in modsById) {
            const existing = modsById[mod.id];
            if(semverGt(mod.version, existing.version)) {
                modsById[mod.id] = mod;
            }
        }   else    {
            modsById[mod.id] = mod;
        }
    });

    return Object.values(modsById)
}