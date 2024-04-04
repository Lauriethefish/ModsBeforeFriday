import { useState } from "react";
import { LogWindow, useLog } from "./LogWindow";
import { Mod } from "../Models";
import { ErrorModal, Modal } from "./Modal";
import { Adb } from '@yume-chan/adb';
import { LogMsg, Mods } from "../Messages";
import { ModCard } from "./ModCard";
import ModIcon from '../icons/mod-icon.svg'
import '../css/ModManager.css';
import { removeMod, setModStatuses } from "../Agent";

interface ModManagerProps {
    mods: Mod[],
    setMods: (mods: Mod[]) => void
    device: Adb
}

export function ModManager(props: ModManagerProps) {
    const { mods, setMods } = props;
    
    const [changes, setChanges] = useState({} as { [id: string]: boolean });
    const [isWorking, setWorking] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [modError, setModError] = useState(null as string | null);
    sortById(mods);

    return <>
        <div className='horizontalCenter'>
            <div className='container horizontalCenter'>
                <h1>Mods</h1>
                <img src={ModIcon} alt="A plug and its socket, disconnected." />
            </div>

            {Object.keys(changes).length > 0 && <div>
                <button id="syncButton" onClick={async () => {
                    setChanges({});
                    console.log("Installing mods, statuses requested: " + JSON.stringify(changes));
                    try {
                        setWorking(true);
                        const updatedMods = await setModStatuses(props.device, changes, addLogEvent);

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
                }}>Sync Changes</button>
            </div>}
        </div>
        {mods.map(mod => <ModCard
            mod={mod}
            key={mod.id}
            onRemoved={async () => {
                setWorking(true);
                try {
                    setMods(await removeMod(props.device, mod.id, addLogEvent));
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
        <Modal isVisible={isWorking}>
            <div className='syncingWindow'>
                <h1>Syncing Mods...</h1>
                <LogWindow events={logEvents} />
            </div>
        </Modal>
        <ErrorModal isVisible={modError != null}
            title={"Failed to sync mods"}
            description={modError!}
            onClose={() => setModError(null)} />
    </>
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