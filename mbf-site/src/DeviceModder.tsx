import { Adb } from '@yume-chan/adb';
import { prepareAgent, runCommand } from "./Agent";
import { useEffect, useState } from 'react';
import { LogMsg, ModStatus } from './Messages';
import './css/DeviceModder.css';
import { ModCard } from './components/ModCard';
import { ReactComponent as ModIcon } from './icons/mod-icon.svg'
import { LogWindow, useLog } from './components/LogWindow';
import { Mod } from './Models';
import { Mods } from './Messages';
import { ErrorModal, Modal } from './components/Modal';

interface DeviceModderProps {
    device: Adb
}

async function loadModStatus(device: Adb): Promise<ModStatus> {
    await prepareAgent(device);

    return await runCommand(device, {
        type: 'GetModStatus'
    }) as ModStatus;
}

async function patchApp(device: Adb, addLogEvent: (event: LogMsg) => void): Promise<Mod[]> {
    let response = await runCommand(device, {
        type: 'Patch'
    }, addLogEvent);

    return (response as Mods).installed_mods;
}

async function setModStatuses(device: Adb, changesRequested: { [id: string]: boolean }, addLogEvent: (event: LogMsg) => void): Promise<Mod[]> {
    let response = await runCommand(device, {
        type: 'SetModsEnabled',
        statuses: changesRequested
    }, addLogEvent);

    return (response as Mods).installed_mods;
}

function DeviceModder(props: DeviceModderProps) {
    const [modStatus, setModStatus] = useState(null as ModStatus | null);

    useEffect(() => {
        loadModStatus(props.device).then(data => setModStatus(data));
    }, [props.device]);

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
    }   else if(modStatus.app_info.is_modded)   {
        return <>
            <div className='container mainContainer'>
                <h1>App is modded</h1>
                <p>Beat Saber is already modded on your Quest, and the version that's installed is compatible with mods.</p>

                <InstallStatus modloaderReady={modStatus.modloader_present} coreModsReady={modStatus.core_mods.all_core_mods_installed} />
            </div>

            <ModManager initialMods={modStatus.installed_mods} device={props.device}/>
        </>
    }   else    {
        return <PatchingMenu
            device={props.device}
            app_version={modStatus.app_info.version}
            onCompleted={mods => {
                console.log("App is now patched, moving into mods menu");
                setModStatus({
                    'type': 'ModStatus',
                    app_info: {
                        is_modded: true,
                        version: modStatus.app_info!.version
                    },
                    core_mods: modStatus.core_mods,
                    modloader_present: true,
                    installed_mods: mods
                });
            }}
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
    modloaderReady: boolean,
    coreModsReady: boolean
}

function InstallStatus(props: InstallStatusProps) {
    if(props.modloaderReady && props.coreModsReady) {
        return <p>Everything should be ready to go! &#9989;</p>
    }   else {
        return <div>
            <h3 className="warning">Problems found with your install:</h3>
            <p>These must be fixed before custom songs will work!</p>
            <ul>
                {!props.modloaderReady && <li>Modloader not found &#10060;</li>}
                {!props.coreModsReady && <li>Core mods missing or out of date &#10060;</li>}
            </ul>
            <button>Fix issues</button>
        </div>
    }
}

interface PatchingMenuProps {
    app_version: string,
    device: Adb,
    onCompleted: (installed_mods: Mod[]) => void
}

function PatchingMenu(props: PatchingMenuProps) {
    const [isPatching, setIsPatching] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [patchingError, setPatchingError] = useState(null as string | null);

    if(!isPatching) {
        return <div className='container mainContainer'>
            <h1>Install Custom Songs</h1>
            <p>Your app has version: {props.app_version}, which is supported by mods!</p>
            <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
            Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>

            <h2 className='warning'>READ CAREFULLY</h2>
            <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
            <b>In addition, by modding the game you will lose access to both vanilla leaderboards and vanilla multiplayer.</b> (Modded leaderboards/servers are available.)

            <button className="modButton" onClick={async () => {
                setIsPatching(true);
                try {
                    props.onCompleted(await patchApp(props.device, addLogEvent));
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

interface ModManagerProps {
    initialMods: Mod[],
    device: Adb
}

function ModManager(props: ModManagerProps) {
    const [mods, setMods] = useState(props.initialMods);
    const [changes, setChanges] = useState({} as { [id: string]: boolean });
    const [isWorking, setWorking] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [modError, setModError] = useState(null as string | null);
    sortById(mods);

    return <>
        <div className='horizontalCenter'>
            <div className='container horizontalCenter'>
                <h1>Mods</h1>
                <ModIcon stroke="white"/>
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
            onEnabledChanged={enabled => {
                const newChanges = { ...changes };
                newChanges[mod.id] = enabled;
                setChanges(newChanges);
            }}/>
        )}
        <Modal isVisible={isWorking}>
            <div id="syncing">
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