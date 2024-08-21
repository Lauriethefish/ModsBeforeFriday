import { useEffect, useState } from "react";
import { VersionedModRepo, ModRepoMod, loadRepo } from "../ModsRepo";
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
    const [modRepo, setModRepo] = useState(null as VersionedModRepo | null);
    const [failedToLoad, setFailedToLoad] = useState(false);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        console.log("Loading mods");
        loadRepo(gameVersion)
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
    }   else {
        return <>
            <div className="mod-list fadeIn">
                {prepareModRepoForDisplay(latestVersions(modRepo), props.existingMods).map(displayInfo => 
                    <ModRepoCard
                            mod={displayInfo.mod}
                            key={displayInfo.mod.id}
                            update={displayInfo.needUpdate}
                            onInstall={() => onDownload(displayInfo.mod.download)} />
                )}
            </div>
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
            needUpdate: existingInstall !== undefined && semverGt(mod.version, existingInstall.version)
                // Core mod updates are handled by the core mod index - do not prompt the user to update a core mod
                // when the update is yet to be pushed to the core mod index.
                && !existingInstall.is_core,
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

// Extracts the latest version of each mod from the provided mods for a given game version.
function latestVersions(modsById: VersionedModRepo): ModRepoMod[] {
    const latestVersions: ModRepoMod[] = [];
    for (const [id, versions] of Object.entries(modsById)) {
        let latestVer: ModRepoMod | null = null;

        // Find the latest version of this mod.
        for (const [version, mod] of Object.entries(versions)) {
            if(latestVer === null || semverGt(version, latestVer.version)) {
                latestVer = mod;
            }
        }

        if(latestVer !== null) {
            latestVersions.push(latestVer);
        }
    }

    return latestVersions;
}