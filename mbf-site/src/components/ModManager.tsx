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
import { toast } from "react-toastify";

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
                    const importResult = await importMod(device, file, addLogEvent);
                    if(importResult.type === 'ImportedFileCopy') {
                        console.log("Successfully copied " + file.name + " to " + importResult.copied_to + " due to request from " + importResult.mod_id);
                        toast("Successfully copied " + file.name + " to the path specified by " + importResult.mod_id);
                    }   else if(importResult.type === 'ImportedSong') {
                        toast("Successfully imported song " + file.name);
                    }   else    {
                        // Don't install a mod by default if its version mismatches: we want the user to understand the consequences
                        const { installed_mods, imported_id } = importResult;

                        const imported_mod = installed_mods.find(mod => mod.id === imported_id)!;
                        const versionMismatch = gameVersion !== null && gameVersion !== imported_mod.game_version;

                        setMods(installed_mods);
                        if(versionMismatch) {
                            setModError("The mod `" + imported_id + "` was not enabled automatically as it is not designed for game version v" + gameVersion + ".");
                        }   else    {
                            setMods(await setModStatuses(device, { [imported_id]: true }, addLogEvent));
                            toast("Successfully downloaded and installed " + imported_id + " v" + imported_mod.version)
                        }
                    }
                }   catch(e)   {
                    setModError("Failed to import file: " + e);
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
        <Modal isVisible={isWorking}>
            <div className='syncingWindow'>
                <h1>Syncing Mods...</h1>
                <LogWindow events={logEvents} />
            </div>
        </Modal>
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
                ref={inputFile}
                style={{display: 'none'}}
                onChange={ev => {
                    const files = ev.target.files;
                    if(files !== null) {
                        onUploaded(files[0]);
                    }
                }}
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