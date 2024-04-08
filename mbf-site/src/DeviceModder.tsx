import { Adb } from '@yume-chan/adb';
import { loadModStatus, patchApp, quickFix } from "./Agent";
import { useEffect, useState } from 'react';
import { ModLoader, ModStatus } from './Messages';
import './css/DeviceModder.css';
import { LogWindow, useLog } from './components/LogWindow';
import { ErrorModal, Modal } from './components/Modal';
import { ModManager } from './components/ModManager';
import { trimGameVersion } from './Models';

interface DeviceModderProps {
    device: Adb,
    // Quits back to the main menu, optionally giving an error that caused the quit.
    quit: (err: unknown | null) => void
}

export async function uninstallBeatSaber(device: Adb) {
    await device.subprocess.spawnAndWait("pm uninstall com.beatgames.beatsaber");
}

export function DeviceModder(props: DeviceModderProps) {
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


                    <h4>Not sure what to do next?</h4>
                    <NextSteps />
                </div>

                <ModManager mods={modStatus.installed_mods}
                    setMods={mods => setModStatus({ ...modStatus, installed_mods: mods })}
                    device={device}
                    gameVersion={modStatus.app_info.version}
                    quit={() => quit(undefined)}
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
        <p>You have Beat Saber v{trimGameVersion(props.version)} installed, but this version has no support for mods!</p>
        <p>To install custom songs, one of the following versions is needed:</p>
        <ul>
            {props.supportedVersions.map(ver => <li key={ver}>{trimGameVersion(ver)}</li>)}
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
            <p className='warning'>You must not disconnect your device during this process.</p>
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

function NextSteps() {
    return <ul>
        <li>Load up the game and look left. A menu should be visible that shows your mods.</li>
        <li>Click the <b>"SongDownloader"</b> mod and browse for custom songs in-game.</li>
        <li>Download additional mods below!</li>
    </ul>
}