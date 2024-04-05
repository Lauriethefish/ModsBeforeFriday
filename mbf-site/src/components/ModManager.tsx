import { useRef, useState } from "react";
import { LogWindow, useLog } from "./LogWindow";
import { Mod } from "../Models";
import { YesNoModal, ErrorModal, Modal } from "./Modal";
import { Adb } from '@yume-chan/adb';
import { ModCard } from "./ModCard";
import ModIcon from '../icons/mod-icon.svg'
import UploadIcon from '../icons/upload.svg';
import '../css/ModManager.css';
import { importMod, removeMod, setModStatuses } from "../Agent";

interface ModManagerProps {
    mods: Mod[],
    gameVersion: string,
    setMods: (mods: Mod[]) => void
    device: Adb
}

export function ModManager(props: ModManagerProps) {
    const { mods, setMods, device, gameVersion } = props;
    
    const [changes, setChanges] = useState({} as { [id: string]: boolean });
    const [isWorking, setWorking] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [modError, setModError] = useState(null as string | null);
    sortById(mods);

    const hasChanges = Object.keys(changes).length > 0;

    return <>
        <div className="horizontalCenter">
            <Title />
            <div>
            {hasChanges &&
            <button id="syncButton"
                onClick={async () => {
                    setChanges({});
                    console.log("Installing mods, statuses requested: " + JSON.stringify(changes));
                    try {
                        setWorking(true);
                        const updatedMods = await setModStatuses(device, changes, addLogEvent);

                        let allSuccesful = true;
                        updatedMods.forEach(m => {
                            if(m.id in changes && m.is_enabled !== changes[m.id]) {
                                allSuccesful = false;
                            }
                        })
                        setMods(updatedMods);

                        if(!allSuccesful) {
                            setModError("Not all the selected mods were successfully installed/uninstalled."
                            + "\nThis happens because two changes were made that conflict, e.g. trying to install a mod but uninstall one of its dependencies.");
                        }
                    }   catch(e) {
                        setModError(String(e));
                    }  finally {
                        setWorking(false);
                    }
            }}>Sync Changes</button>}
            {!hasChanges && <UploadButton onUploaded={async file => {
                console.log("Importing " + file.name);
                try {
                    setWorking(true);
                    const { installed_mods, imported_id } = await importMod(device, file, addLogEvent);

                    // Don't install a mod by default if its version mismatches: we 
                    const versionMismatch = gameVersion !== null 
                        && gameVersion != installed_mods.find(mod => mod.id == imported_id)?.version;
                    if(versionMismatch) {
                        setMods(installed_mods);
                        setModError("The mod `" + imported_id + "` was not enabled automatically as it is not designed for game version v" + gameVersion + ".");
                    }   else    {
                        setMods(await setModStatuses(device, { [imported_id]: true }, addLogEvent));
                    }
                }   catch(e)   {
                    setModError("Failed to import mod: " + e);
                }   finally {
                    setWorking(false);
                }
            }} />}
            </div>
        </div>
        {mods.map(mod => <ModCard
            gameVersion={gameVersion}
            mod={mod}
            key={mod.id}
            onRemoved={async () => {
                setWorking(true);
                try {
                    setMods(await removeMod(device, mod.id, addLogEvent));
                }   catch(e) {
                    setModError(String(e));
                }   finally {
                    setWorking(false);
                }
            }}
            onEnabledChanged={enabled => {
                const newChanges = { ...changes };
                newChanges[mod.id] = enabled;
                setChanges(newChanges);
            }}/>
        )}
        <ErrorModal isVisible={modError != null}
            title={"Failed to sync mods"}
            description={modError!}
            onClose={() => setModError(null)} />
    </>
}

function Title() {
    return <div className='container'>
        <div className="horizontalCenter">
            <h1>Mods</h1>
            <img src={ModIcon} />
        </div>
    </div>
}

interface UploadButtonProps {
    onUploaded: (file: File) => void;
}

function UploadButton(props: UploadButtonProps) {
    const { onUploaded } = props;

    const inputFile = useRef<HTMLInputElement | null>(null);

    return <button id="uploadButton" onClick={() => inputFile.current?.click()}>
            Upload
            <img src={UploadIcon}/>
            <input type="file"
                id="file"
                accept=".qmod"
                ref={inputFile}
                style={{display: 'none'}}
                onChange={ev => onUploaded(ev.target.files![0])}
            />
        </button>
}

function sortById(mods: Mod[]) {
    mods.sort((a, b) => {
        if(a.id > b.id) {
            return 1;
        }   else if(a.id < b.id) {
            return -1;
        }   else    {
            return 0;
        }
    })
}