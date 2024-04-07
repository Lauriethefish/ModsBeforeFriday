import { useEffect, useState } from "react";
import { ModRepo, ModRepoMod, loadRepo } from "../ModsRepo";
import { ModRepoCard } from "./ModRepoCard";
import { toast } from "react-toastify";
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
            return <p>
                <h1>Failed to load mods</h1>
                <p>Please check that your internet is working.</p>
                <button onClick={() => {
                    setAttempt(attempt + 1);
                    setFailedToLoad(false);
                }}>Try again</button>
            </p>
        }   else    {
            return <h1>Loading mods...</h1>
        }
    }
    else if(gameVersion in modRepo) {
        return <>
            <div className="mod-list">
                {latestVersions(modRepo[gameVersion]).map(mod => {
                    const existingInstall = props.existingMods
                    .find(existing => existing.id === mod.id);

                    if(existingInstall === undefined || existingInstall?.version !== mod.version) {
                        return <ModRepoCard
                            mod={mod}
                            key={mod.id}
                            update={existingInstall !== undefined}
                            onInstall={() => onDownload(mod.download)} />
                    }
                })}
            </div>
        </>
    }   else    {
        return <>
            <h1>Failed to load mods</h1>
            <p>No mods are available for v{props.gameVersion} of the game!</p>
        </>
    }
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