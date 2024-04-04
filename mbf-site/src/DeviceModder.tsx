import { Adb } from '@yume-chan/adb';
import { prepareAgent, runCommand } from "./Agent";
import { useEffect, useState } from 'react';
import { LogMsg, ModLoader, ModStatus } from './Messages';
import './css/DeviceModder.css';
import { ModCard } from './components/ModCard';
import ModIcon from './icons/mod-icon.svg'
import { LogWindow, useLog } from './components/LogWindow';
import { Mod } from './Models';
import { Mods } from './Messages';
import { ErrorModal, Modal } from './components/Modal';

interface DeviceModderProps {
    device: Adb,
    // Quits back to the main menu, optionally giving an error that caused the quit.
    quit: (err: unknown | null) => void
}

// Gets the status of mods from the quest, i.e. whether the app is patched, and what mods are currently installed.
async function loadModStatus(device: Adb): Promise<ModStatus> {
    await prepareAgent(device);

    return await runCommand(device, {
        type: 'GetModStatus'
    }) as ModStatus;
}

// Instructs the agent to patch the app, adding the modloader and installing the core mods.
// Updates the ModStatus `beforePatch` to reflect the state of the installation after patching.
// (will not patch if the APK is already modded - will just extract the modloader and install core mods.)
async function patchApp(device: Adb,
    beforePatch: ModStatus,
    addLogEvent: (event: LogMsg) => void): Promise<ModStatus> {
    let response = await runCommand(device, {
        type: 'Patch'
    }, addLogEvent);

    // Return the new mod status assumed after patching
    // (patching should fail if any of this is not the case)
    return {
        'type': 'ModStatus',
        app_info: {
            loader_installed: 'Scotland2',
            version: beforePatch.app_info!.version
        },
        core_mods: {
            all_core_mods_installed: true,
            supported_versions: beforePatch.core_mods!.supported_versions
        },
        modloader_present: true,
        installed_mods: (response as Mods).installed_mods
    };
}

async function quickFix(device: Adb,
    beforeFix: ModStatus,
    addLogEvent: (event: LogMsg) => void): Promise<ModStatus> {
    let response = await runCommand(device, {
        type: 'QuickFix'
    }, addLogEvent);

    // Update the mod status to reflect the fixed installation
    return {
        'type': 'ModStatus',
        app_info: beforeFix.app_info,
        core_mods: {
            all_core_mods_installed: true,
            supported_versions: beforeFix.core_mods!.supported_versions,
        },
        installed_mods: (response as Mods).installed_mods,
        modloader_present: true
    }
}

async function setModStatuses(device: Adb, changesRequested: { [id: string]: boolean }, addLogEvent: (event: LogMsg) => void): Promise<Mod[]> {
    let response = await runCommand(device, {
        type: 'SetModsEnabled',
        statuses: changesRequested
    }, addLogEvent);

    return (response as Mods).installed_mods;
}

async function removeMod(device: Adb, mod_id: string, addLogEvent: (event: LogMsg) => void) {
    let response = await runCommand(device, {
        type: 'RemoveMod',
        id: mod_id
    }, addLogEvent);

    return (response as Mods).installed_mods;
}

async function uninstallBeatSaber(device: Adb) {
    await device.subprocess.spawnAndWait("pm uninstall com.beatgames.beatsaber");
}

function DeviceModder(props: DeviceModderProps) {
    const [modStatus, setModStatus] = useState(null as ModStatus | null);
    const { device, quit } = props;
    useEffect(() => {
        loadModStatus(device)
            .then(data => setModStatus(data))
            .catch(err => quit(err));
    }, [device, quit]);

    if(modStatus === null) {
        return <div className='container mainContainer'>
            <h2>Checking Beat Saber installation</h2>
        </div>
    }   else if(modStatus.app_info === null) {
        return <div className='container mainContainer'>
            <h1>Beat Saber is not installed</h1>
            <p>Please install Beat Saber from the store and then refresh the page.</p>
        </div>
    }   else if (modStatus.core_mods === null) {
        return <div className='container mainContainer'>
            <h1>No internet</h1>
            <p>It seems as though <b>your Quest</b> has no internet connection.</p>
            <p>To mod Beat Saber, MBF needs to download files such as a mod loader and several essential mods. 
                <br />This occurs on your Quest's connection. Please make sure that WiFi is enabled, then refresh the page.</p>
        </div>
    }   else if(!(modStatus.core_mods.supported_versions.includes(modStatus.app_info.version))) {
        return <NotSupported version={modStatus.app_info.version} supportedVersions={modStatus.core_mods.supported_versions}/>
    }   else if(modStatus.app_info.loader_installed !== null)   {
        let loader = modStatus.app_info.loader_installed;
        if(loader === 'Scotland2') {
            return <>
                <div className='container mainContainer'>
                    <h1>App is modded</h1>
                    <p>Beat Saber is already modded on your Quest, and the version that's installed is compatible with mods.</p>

                    <InstallStatus
                        modStatus={modStatus}
                        device={device}
                        onFixed={status => setModStatus(status)}
                    />
                </div>

                <ModManager mods={modStatus.installed_mods}
                    setMods={mods => setModStatus({ ...modStatus, installed_mods: mods })}
                    device={device} 
                />
            </>
        }   else    {
            return <IncompatibleLoader device={device} loader={loader} quit={() => quit(null)} />
        }
    }   else   {
        return <PatchingMenu
            device={device}
            modStatus={modStatus}
            onCompleted={modStatus => setModStatus(modStatus)}
        />
    }
}

interface NotSupportedProps {
    version: string,
    supportedVersions: string[]
}

function NotSupported(props: NotSupportedProps) {
    return <div className='container mainContainer'>
        <h1>Unsupported Version</h1>
        <p>You have Beat Saber v{props.version} installed, but this version has no support for mods!</p>
        <p>To install custom songs, one of the following versions is needed:</p>
        <ul>
            {props.supportedVersions.map(ver => <li>{ver}</li>)}
        </ul>
    </div>
}

interface InstallStatusProps {
    modStatus: ModStatus
    onFixed: (newStatus: ModStatus) => void,
    device: Adb
}

function InstallStatus(props: InstallStatusProps) {
    const { modStatus, onFixed, device } = props;

    const [logEvents, addLogEvent] = useLog();
    const [error, setError] = useState(null as string | null);
    const [fixing, setFixing] = useState(false);

    if(modStatus.modloader_present && modStatus.core_mods?.all_core_mods_installed) {
        return <p>Everything should be ready to go! &#9989;</p>
    }   else {
        return <div>
            <h3 className="warning">Problems found with your install:</h3>
            <p>These must be fixed before custom songs will work!</p>
            <ul>
                {!modStatus.modloader_present && 
                    <li>Modloader not found &#10060;</li>}
                {!modStatus.core_mods?.all_core_mods_installed && 
                    <li>Core mods missing or out of date &#10060;</li>}
            </ul>
            <button onClick={async () => {
                try {
                    setFixing(true);
                    onFixed(await quickFix(device, modStatus, addLogEvent));
                }   catch(e) {
                    setError(String(e));
                }   finally {
                    setFixing(false);
                }
            }}>Fix issues</button>

            <Modal isVisible={fixing}>
                <div className='syncingWindow'>
                    <h1>Fixing issues</h1>
                    <LogWindow events={logEvents} />
                </div>
            </Modal>

            <ErrorModal title="Failed to fix issues"
                description={error!}
                isVisible={error != null}
                onClose={() => setError(null)} />
        </div>
    }
}

interface PatchingMenuProps {
    modStatus: ModStatus
    device: Adb,
    onCompleted: (newStatus: ModStatus) => void
}

function PatchingMenu(props: PatchingMenuProps) {
    const [isPatching, setIsPatching] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [patchingError, setPatchingError] = useState(null as string | null);

    const { onCompleted, modStatus, device } = props;
    if(!isPatching) {
        return <div className='container mainContainer'>
            <h1>Install Custom Songs</h1>
            <p>Your app has version: {props.modStatus.app_info?.version}, which is supported by mods!</p>
            <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
            Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>

            <h2 className='warning'>READ CAREFULLY</h2>
            <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
            <b>In addition, by modding the game you will lose access to both vanilla leaderboards and vanilla multiplayer.</b> (Modded leaderboards/servers are available.)

            <button className="modButton" onClick={async () => {
                setIsPatching(true);
                try {
                    onCompleted(await patchApp(device, modStatus, addLogEvent));
                } catch(e) {
                    setPatchingError(String(e));
                    setIsPatching(false);
                }
            }}>Mod the app</button>

            <ErrorModal
                isVisible={patchingError != null}
                title={"Failed to install mods"}
                description={'An error occured while patching ' + patchingError}
                onClose={() => setPatchingError(null)}/>
        </div>
    }   else    {
        return <div className='container mainContainer'>
            <h1>App is being patched</h1>
            <p>This should only take a few minutes, but might take up to 10 on a very slow internet connection.</p>
            <LogWindow events={logEvents}/>
        </div>
    }
}

interface IncompatibleLoaderProps {
    loader: ModLoader,
    device: Adb,
    quit: () => void
}

function IncompatibleLoader(props: IncompatibleLoaderProps) {
    const { loader, device, quit } = props;
    return <div className='container mainContainer'>
        <h1>Incompatible Modloader</h1>
        <p>Your app is patched with {loader === 'QuestLoader' ? "the QuestLoader" : "an unknown"} modloader, which isn't supported by MBF.</p>
        <p>You will need to uninstall your app and reinstall the latest vanilla version so that the app can be re-patched with Scotland2.</p>
        <p>Do not be alarmed! Your custom songs will not be lost.</p>

        <button onClick={async () => {
            await uninstallBeatSaber(device);
            quit();
        }}>Uninstall Beat Saber</button>
    </div>
}

interface ModManagerProps {
    mods: Mod[],
    setMods: (mods: Mod[]) => void
    device: Adb
}

function ModManager(props: ModManagerProps) {
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

export default DeviceModder;