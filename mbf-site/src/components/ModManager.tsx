import { useRef, useState } from "react";
import { Mod, trimGameVersion } from "../Models";
import { Modal } from "./Modal";
import { Adb } from '@yume-chan/adb';
import { ModCard } from "./ModCard";
import UploadIcon from '../icons/upload.svg';
import ToolsIcon from '../icons/tools-icon.svg';
import '../css/ModManager.css';
import { importFile, importUrl, removeMod, setModStatuses } from "../Agent";
import { toast } from "react-toastify";
import { ModRepoBrowser } from "./ModRepoBrowser";
import { ImportResult, ImportedMod, ModStatus } from "../Messages";
import { OptionsMenu } from "./OptionsMenu";
import useFileDropper from "../hooks/useFileDropper";
import { Log } from "../Logging";
import { useSetWorking, useSyncStore, wrapOperation } from "../SyncStore";
import { ModRepoMod } from "../ModsRepo";
import SyncIcon from "../icons/sync.svg"

interface ModManagerProps {
    gameVersion: string,
    setMods: (mods: Mod[]) => void,
    modStatus: ModStatus,
    setModStatus: (status: ModStatus) => void,
    device: Adb,
    quit: (err: unknown) => void
}

enum SelectedMenu {
    add, current, options
}

export function ModManager(props: ModManagerProps) {
    const { modStatus, setModStatus, setMods, device, gameVersion, quit } = props;
    const mods = modStatus.installed_mods;
    const [menu, setMenu] = useState(SelectedMenu.add as SelectedMenu);

    sortByNameAndIfCore(mods);

    return <>
        <Title menu={menu} setMenu={setMenu}/>
        
        {/* We use a style with display: none when hiding this menu, as this avoids remounting the component,
            which would fetch the mods index again. */}
        <AddModsMenu
            mods={mods}
            setMods={setMods}
            gameVersion={gameVersion}
            device={device}
            visible={menu === SelectedMenu.add}
        />
        
        <InstalledModsMenu
            mods={mods}
            setMods={setMods}
            gameVersion={gameVersion}
            device={device}
            visible={menu === SelectedMenu.current}
        />
        
        <OptionsMenu
            device={device}
            quit={quit}
            modStatus={modStatus}
            setModStatus={setModStatus}
            visible={menu === SelectedMenu.options}
        />
    </>
}

interface TitleProps {
    menu: SelectedMenu,
    setMenu: (menu: SelectedMenu) => void
}

function Title(props: TitleProps) {
    const { menu, setMenu } = props;

    return <div className='container noPadding horizontalCenter sticky coverScreen'>
        <div className={`tab-header ${menu === SelectedMenu.current ? "selected":""}`}
            onClick={() => setMenu(SelectedMenu.current)}>
            <h1>Your Mods</h1>
        </div>
        <span className={`tab-header settingsCog ${menu === SelectedMenu.options ? "selected":""}`}
            onClick={() => setMenu(SelectedMenu.options)}>
            <img src={ToolsIcon} />
        </span>
        <div className={`tab-header ${menu === SelectedMenu.add ? "selected":""}`}
            onClick={() => setMenu(SelectedMenu.add)}>
            <h1>Add Mods</h1>
        </div>
    </div>
}

interface ModMenuProps {
    mods: Mod[],
    setMods: (mods: Mod[]) => void,
    gameVersion: string,
    device: Adb
    visible?: boolean
}

function InstalledModsMenu(props: ModMenuProps) {
    const { mods,
        setMods,
        gameVersion,
        device
    } = props;

    const [changes, setChanges] = useState({} as { [id: string]: boolean });

    return <div className={`installedModsMenu fadeIn ${props.visible ? "" : "hidden"}`}>
        {Object.keys(changes).length > 0 && <button className={`syncChanges fadeIn ${props.visible ? "" : "hidden"}`} onClick={async () => {
            setChanges({});
            Log.debug("Installing mods, statuses requested: " + JSON.stringify(changes));
            await wrapOperation("Syncing mods", "Failed to sync mods", async () => {
                const modSyncResult = await setModStatuses(device, changes);
                setMods(modSyncResult.installed_mods);

                if(modSyncResult.failures !== null) {
                    throw modSyncResult.failures;
                }
            });

        }}>
            Sync Changes
            <img src={SyncIcon} alt="Sync Icon" />
        </button>}

		<div className="mod-list">
			{mods.map(mod => <ModCard
				gameVersion={gameVersion}
				mod={mod}
                pendingChange={changes[mod.id]}
				key={mod.id}
				onRemoved={async () => {
                    await wrapOperation("Removing mod", "Failed to remove mod", async () => {
						setMods(await removeMod(device, mod.id));
                        const newChanges = { ...changes };
                        
                        delete newChanges[mod.id];
                        
                        setChanges(newChanges);
                    });
				}}
				onEnabledChanged={enabled => {
					const newChanges = { ...changes };

                    if (changes[mod.id] !== undefined) {
                        // If the mod is in the original state, remove it from the changes
                        delete newChanges[mod.id];
                    }   else    {
                        // Otherwise, add it to the changes
                        newChanges[mod.id] = enabled;
                    }

					setChanges(newChanges);
				}}/>
			)}
		</div>
    </div>
}

function UploadButton({ onUploaded }: { onUploaded: (files: File[]) => void}) {
    const inputFile = useRef<HTMLInputElement | null>(null);
    return <button id="uploadButton" onClick={() => inputFile.current?.click()} title="Upload any .QMOD file, any song as a .ZIP, any Qosmetics files or any other file accepted by a particular mod.">
        Upload Files
        <img src={UploadIcon}/>
        <input type="file"
            id="file"
            multiple={true}
            ref={inputFile}
            style={{display: 'none'}}
            onChange={ev => {
                const files = ev.target.files;
                if(files !== null) {
                    onUploaded(Array.from(files));
                }
                ev.target.value = "";
            }}
        />
    </button>
}


type ImportType = "Url" | "File" | "ModRepo";
interface QueuedImport {
    type: ImportType
}

interface QueuedFileImport extends QueuedImport {
    file: File,
    type: "File"
}

interface QueuedUrlImport extends QueuedImport {
    url: string,
    type: "Url"
}

interface QueuedModRepoImport extends QueuedImport {
    mod: ModRepoMod,
    type: 'ModRepo',
}

const importQueue: QueuedImport[] = [];
let isProcessingQueue: boolean = false;

function AddModsMenu(props: ModMenuProps) {
    const {
        mods,
        setMods,
        gameVersion,
        device
    } = props;

    // Automatically installs a mod when it is imported, or warns the user if it isn't designed for the current game version.
    // Gives appropriate toasts/reports errors in each case.
    async function onModImported(result: ImportedMod) {
        const { installed_mods, imported_id } = result;
        setMods(installed_mods);

        const imported_mod = installed_mods.find(mod => mod.id === imported_id)!;
        const versionMismatch = imported_mod.game_version !== null &&gameVersion !== imported_mod.game_version;
        if(versionMismatch) {
            // Don't install a mod by default if its version mismatches: we want the user to understand the consequences
            toast.error("The mod `" + imported_id + "` was not enabled automatically as it is not designed for game version v" 
                + trimGameVersion(gameVersion) + ".", { autoClose: false });
        }   else    {
            try {
                const result = await setModStatuses(device, { [imported_id]: true });
                setMods(result.installed_mods);

                // This is where typical mod install failures occur
                if (result.failures !== null) {
                    toast.error(result.failures, { autoClose: false });
                }   else    {
                    toast.success("Successfully downloaded and installed " + imported_mod.name + " v" + imported_mod.version)
                }

            }   catch(err) {
                // If this occurs, it's a panic i.e. bug in the agent
                toast.error(`Failed to install ${imported_id} after importing due to an internal error: ${err}`, { autoClose: false} );
            }
        }
    }

    // Processes an ImportResult
    async function onImportResult(importResult: ImportResult) {
        const filename = importResult.used_filename;
        const typedResult = importResult.result;
        if(typedResult.type === 'ImportedFileCopy') {
            Log.info("Successfully copied " + filename + " to " + typedResult.copied_to + " due to request from " + typedResult.mod_id);
            toast.success("Successfully copied " + filename + " to the path specified by " + typedResult.mod_id);
        }   else if(typedResult.type === 'ImportedSong') {
            toast.success("Successfully imported song " + filename);
        }   else if(typedResult.type === 'NonQuestModDetected')  {
            toast.error(`${importResult.used_filename} is a PC mod, with the .DLL file extension. You can only install Quest mods with the .QMOD file extension. Get these from the 'Add Mods' tab.`, { autoClose: false })
        }   else    {
            await onModImported(typedResult);
        }
    }

    async function handleFileImport(file: File) {
        try {
            const importResult = await importFile(device, file);
            await onImportResult(importResult);
        }   catch(e)   {
            toast.error("Failed to import file: " + e);
        }
    }

    async function handleUrlImport(url: string) {
        if (url.startsWith("file:///")) {
            toast.error("Cannot process dropped file from this source, drag from the file picker instead. (Drag from OperaGX file downloads popup does not work)");
            return;
        }
        try {
            const importResult = await importUrl(device, url)
            await onImportResult(importResult);
        }   catch(e)   {
            toast.error(`Failed to import file: ${e}`);
        }
    }

    async function enqueueImports(imports: QueuedImport[]) {
        // Add the new imports to the queue
        importQueue.push(...imports);
        // If somebody else is processing the queue already, stop and let them finish processing the whole queue.
        if(isProcessingQueue) {
            return;
        }
        
        // Otherwise, we must stop being lazy and process the queue ourselves.
        Log.debug("Now processing import queue");
        isProcessingQueue = true;

        let disconnected = false;
        device.disconnected.then(() => disconnected = true);
        const setWorking = useSetWorking("Importing");
        const { setStatusText } = useSyncStore.getState(); 

        setWorking(true);
        while(importQueue.length > 0 && !disconnected) {
            // Process the next import, depending on if it is a URL or file
            const newImport = importQueue.pop()!;

            if(newImport.type == "File") {
                const file = (newImport as QueuedFileImport).file;
                setStatusText(`Processing file ${file.name}`);
                await handleFileImport(file);
            }   else if(newImport.type == "Url") {
                const url = (newImport as QueuedUrlImport).url;
                setStatusText(`Processing url ${url}`);

                await handleUrlImport(url);
            }   else if(newImport.type == "ModRepo") {
                const mod = (newImport as QueuedModRepoImport).mod;

                setStatusText(`Installing ${mod.name} v${mod.version}`);

                await handleUrlImport(mod.download);
            }
        }
        setWorking(false);
        isProcessingQueue = false;
    }

    const { isDragging } = useFileDropper({
        onFilesDropped: async files => {
            enqueueImports(files
                .map(file => {
                return { type: "File", file: file };
            }))
        },
        onUrlDropped: async url => {
            const urlImport: QueuedUrlImport = {
                type: "Url",
                url: url
            };
            enqueueImports([urlImport])
        }
    })

    return <div className={`verticalCenter ${props.visible ? "" : "hidden"}`}>
        <Modal isVisible={isDragging}>
            <div className="horizontalCenter">
                <img src={UploadIcon}/>
                <h1>Drag 'n' drop files or links!</h1>
            </div>
        </Modal>

        <UploadButton onUploaded={async files => await enqueueImports(files.map(file => {
                return { type: "File", file: file };
            }))} />

        <ModRepoBrowser existingMods={mods} gameVersion={gameVersion} visible={props.visible} onDownload={async mods => {
            const modRepoImports: QueuedModRepoImport[] = mods.map(mod => { return {
                type: "ModRepo",
                mod
            }});
            enqueueImports(modRepoImports);
        }} />
    </div>
}


// Sorts mods by their ID alphabetically
// Also sorts the mods so that core mods come last in the list.
function sortByNameAndIfCore(mods: Mod[]) {
    mods.sort((a, b) => {
        // Sort core mods after other mods
        // This is so that user-installed mods are more obvious in the list.
        if(!b.is_core && a.is_core) {
            return 1;
        }   else if(!a.is_core && b.is_core) {
            return -1;
        }
        
        const nameA = a.name.toLowerCase().trim();
        const nameB = b.name.toLowerCase().trim();

        if(nameA > nameB) {
            return 1;
        }   else if(nameA < nameB) {
            return -1;
        }   else    {
            return 0;
        }
    })
}
