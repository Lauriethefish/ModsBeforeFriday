import { useEffect, useState } from "react";
import { VersionedModRepo, ModRepoMod, loadRepo } from "../ModsRepo";
import { ModRepoCard } from "./ModRepoCard";
import { gt as semverGt } from "semver";
import { Mod } from "../Models";
import { Log } from "../Logging";
import '../css/ModRepoBrowser.css';
import DownloadIcon from '../icons/download-icon.svg';
import UpdateIcon from '../icons/update-icon.svg';

interface ModRepoBrowserProps {
    gameVersion: string,
    onDownload: (urls: ModRepoMod[]) => void,
    existingMods: Mod[]
    visible?: boolean
}

export function ModRepoBrowser(props: ModRepoBrowserProps) {
    const { gameVersion, onDownload } = props;
    const [modRepo, setModRepo] = useState(null as VersionedModRepo | null);
    const [failedToLoad, setFailedToLoad] = useState(false);
    const [attempt, setAttempt] = useState(0);
    const [flagged, setFlagged] = useState([] as ModDisplayInfo[]);

    // Removes a mod from the list of flagged mods
    function unflag(displayInfo: ModDisplayInfo) {
        setFlagged(flagged.filter(mod => mod.mod.id != displayInfo.mod.id))
    }
    
    useEffect(() => {
        (async () => {
            Log.debug("Loading mods");

            try {
                const gameRepo = await loadRepo(gameVersion);
                const globalRepo = (await loadRepo("global"));

                // Iterate and mark all global mods as global.
                for (const packages of Object.values(globalRepo)) {
                    for (const mod of Object.values(packages)) {
                        mod.global = true;
                    }
                }

                const combinedRepo = { ...globalRepo, ...gameRepo };

                // Initially flag outdated mods for update
                // NB: Currently this means that we prepare the mod repo for display twice, i.e. once just after
                // loading it to flag mods needing updates and once before each render of the mod repo browser.
                //
                // We cannot just do it once here since each change to the installed mods requires properties such as
                // whether a mod even needs an update or install to be changed.
                const displayMods = prepareModRepoForDisplay(
                    latestVersions(combinedRepo),
                    props.existingMods
                );
                setFlagged(displayMods.filter((mod) => mod.needUpdate));
                setModRepo(combinedRepo);
            } catch (e) {
                setFailedToLoad(true);
            }
        })();
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
        const hasFlaggedNewMods = flagged.filter(mod => !mod.needUpdate).length > 0;
        const hasFlaggedModsToUpdate = flagged.filter(mod => mod.needUpdate).length > 0;

        return <>
            {flagged.length > 0 && 
                <button className={`installMarked fadeIn ${props.visible ? "" : "hidden"}`} onClick={() => {
                    onDownload(flagged.map(mod => mod.mod));
                    setFlagged([]);
                }}>
                    {hasFlaggedModsToUpdate && hasFlaggedNewMods && "Install/Update "}
                    {hasFlaggedModsToUpdate && !hasFlaggedNewMods && "Update "}
                    {!hasFlaggedModsToUpdate && hasFlaggedNewMods && "Install "}
                    {flagged.length} {flagged.length > 1 ? "mods" : "mod"}
                    <img src={hasFlaggedNewMods ? DownloadIcon : UpdateIcon} alt="A download icon" />
                </button>}

            <div className="mod-list fadeIn">
                {prepareModRepoForDisplay(latestVersions(modRepo), props.existingMods).map(displayInfo => 
                    <ModRepoCard
                            mod={displayInfo.mod}
                            key={displayInfo.mod.id}
                            update={displayInfo.needUpdate}
                            onInstall={() => {
                                onDownload([displayInfo.mod]);
                                unflag(displayInfo);
                            }}
                            isFlagged={flagged.find(mod => mod.mod.id === displayInfo.mod.id) !== undefined}
                            setFlagged={isFlagged => {
                                if(isFlagged) {
                                    setFlagged([...flagged, displayInfo]);
                                }   else    {
                                    unflag(displayInfo);
                                }
                            }} />
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
    }).filter(mod => mod.needUpdate || (!mod.alreadyInstalled && !mod.mod.global)) // Skip any mods that are already installed and up to date, or global mods
    .sort((a, b) => {
        // Show mods that need an update first in the list
        if(!a.needUpdate && b.needUpdate) {
            return 1;
        }

        if(!b.needUpdate && a.needUpdate) {
            return -1;
        }

        const nameA = a.mod.name.toLowerCase().trim();
        const nameB = b.mod.name.toLowerCase().trim();

        // Sort the rest of the mods alphabetically
        if(nameA > nameB) {
            return 1;
        }   else if(nameA < nameB) {
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