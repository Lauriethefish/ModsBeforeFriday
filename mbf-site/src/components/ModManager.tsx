import { useRef, useState } from "react";
import { LogWindow, useLog } from "./LogWindow";
import { Mod, trimGameVersion } from "../Models";
import { ErrorModal, Modal } from "./Modal";
import { Adb } from '@yume-chan/adb';
import { ModCard } from "./ModCard";
import UploadIcon from '../icons/upload.svg';
import ToolsIcon from '../icons/tools-icon.svg';
import '../css/ModManager.css';
import { LogEventSink, importFile, importModUrl, removeMod, setModStatuses } from "../Agent";
import { toast } from "react-toastify";
import { ModRepoBrowser } from "./ModRepoBrowser";
import { ImportedMod, ModStatus } from "../Messages";
import { OptionsMenu } from "./OptionsMenu";

interface ModManagerProps {
    gameVersion: string,
    setMods: (mods: Mod[]) => void,
    modStatus: ModStatus,
    device: Adb,
    quit: (err: unknown) => void
}

type SelectedMenu = 'add' | 'current' | 'options';

export function ModManager(props: ModManagerProps) {
    const { modStatus, setMods, device, gameVersion, quit } = props;
    const mods = modStatus.installed_mods;

    const [isWorking, setWorking] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [modError, setModError] = useState(null as string | null);
    const [menu, setMenu] = useState('add' as SelectedMenu)
    sortById(mods);

    return <>
        <Title menu={menu} setMenu={setMenu}/>
        
        {/* We use a style with display: none when hiding this menu, as this avoids remounting the component,
            which would fetch the mods index again. */}
        <div className={menu === 'add' ? "" : "hidden"}>
            <AddModsMenu
                mods={mods}
                setMods={setMods}
                setWorking={working => setWorking(working)}
                gameVersion={gameVersion}
                setError={err => setModError(err)}
                device={device}
                addLogEvent={addLogEvent}
            />
        </div>
        
        {menu === 'current' && <InstalledModsMenu
            mods={mods}
            setMods={setMods}
            setWorking={working => setWorking(working)}
            gameVersion={gameVersion}
            setError={err => setModError(err)}
            device={device}
            addLogEvent={addLogEvent}
        />}
        
        {menu === 'options' && <OptionsMenu
            device={device}
            quit={quit}
            modStatus={modStatus}
         />}
        
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

interface TitleProps {
    menu: SelectedMenu,
    setMenu: (menu: SelectedMenu) => void
}

function Title(props: TitleProps) {
    const { menu, setMenu } = props;

    return <div className='container noPadding horizontalCenter'>
        <div className={`tab-header ${menu === 'current' ? "selected":""}`}>
            <h1 onClick={() => setMenu('current')}>Your Mods</h1>
        </div>
        <span className={`tab-header ${menu === 'options' ? "selected":""}`}>
            <img onClick={() => setMenu('options')} src={ToolsIcon} />
        </span>
        <div className={`tab-header ${menu === 'add' ? "selected":""}`}>
            <h1 onClick={() => setMenu('add')}>Add Mods</h1>
        </div>
    </div>
}

interface ModMenuProps {
    mods: Mod[],
    setMods: (mods: Mod[]) => void,
    gameVersion: string,
    setWorking: (working: boolean) => void,
    setError: (err: string) => void,
    addLogEvent: LogEventSink,
    device: Adb
}

function InstalledModsMenu(props: ModMenuProps) {
    const { mods,
        setMods,
        gameVersion,
        setWorking,
        setError,
        addLogEvent,
        device
    } = props;

    const [changes, setChanges] = useState({} as { [id: string]: boolean });
    const hasChanges = Object.keys(changes).length > 0;

    return <>
        {hasChanges && <button id="syncButton" onClick={async () => {
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
                    setError("Not all the selected mods were successfully installed/uninstalled."
                    + "\nThis happens when two changes are made that conflict, e.g. trying to install a mod but uninstall one of its dependencies.");
                }
            }   catch(e) {
                setError(String(e));
            }  finally {
                setWorking(false);
            }
        }}>Sync Changes</button>}

		<div className="mod-list">
			{mods.map(mod => <ModCard
				gameVersion={gameVersion}
				mod={mod}
				key={mod.id}
				onRemoved={async () => {
					setWorking(true);
					try {
						setMods(await removeMod(device, mod.id, addLogEvent));
					}   catch(e) {
						setError(String(e));
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
		</div>
    </>
}

function UploadButton({ onUploaded }: { onUploaded: (file: File) => void}) {
    const inputFile = useRef<HTMLInputElement | null>(null);
    return <button id="uploadButton" onClick={() => inputFile.current?.click()}>
        Upload Mods or Songs
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


function AddModsMenu(props: ModMenuProps) {
    const {
        mods,
        setMods,
        gameVersion,
        setWorking,
        setError,
        addLogEvent,
        device
    } = props;

    // Automatically installs a mod when it is imported, or warns the user if it isn't designed for the current game version.
    // Gives appropriate toasts/reports errors in each case.
    async function onModImported(result: ImportedMod) {
        const { installed_mods, imported_id } = result;
        setMods(installed_mods);

        const imported_mod = installed_mods.find(mod => mod.id === imported_id)!;
        const versionMismatch = gameVersion !== null && gameVersion !== imported_mod.game_version;
        if(versionMismatch) {
            // Don't install a mod by default if its version mismatches: we want the user to understand the consequences
            setError("The mod `" + imported_id + "` was not enabled automatically as it is not designed for game version v" + trimGameVersion(gameVersion) + ".");
        }   else    {
            setMods(await setModStatuses(device, { [imported_id]: true }, addLogEvent));
            toast("Successfully downloaded and installed " + imported_id + " v" + imported_mod.version)
        }
    }

    return <div className="verticalCenter">
        <UploadButton onUploaded={async file => {
            console.log("Importing " + file.name);
            try {
                setWorking(true);
                const importResult = await importFile(device, file, addLogEvent);
                if(importResult.type === 'ImportedFileCopy') {
                    console.log("Successfully copied " + file.name + " to " + importResult.copied_to + " due to request from " + importResult.mod_id);
                    toast("Successfully copied " + file.name + " to the path specified by " + importResult.mod_id);
                }   else if(importResult.type === 'ImportedSong') {
                    toast("Successfully imported song " + file.name);
                }   else    {
                    onModImported(importResult);
                }
            }   catch(e)   {
                setError("Failed to import file: " + e);
            }   finally {
                setWorking(false);
            }
        }} />

        <ModRepoBrowser existingMods={mods} gameVersion={gameVersion} onDownload={async url => {
            setWorking(true);
            try {
                await onModImported(await importModUrl(device, url, addLogEvent));
            }   catch(e) { 
                setError("Failed to install mod " + e);
            }   finally {
                setWorking(false);
            }
        }} />
    </div>
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
