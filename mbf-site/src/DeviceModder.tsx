import { Adb } from '@yume-chan/adb';
import { loadModStatus, patchApp, quickFix } from "./Agent";
import { useEffect, useState } from 'react';
import { ModLoader, ModStatus } from './Messages';
import './css/DeviceModder.css';
import { LogWindow, useLog } from './components/LogWindow';
import { ErrorModal, Modal } from './components/Modal';
import { ModManager } from './components/ModManager';
import { ManifestMod, trimGameVersion } from './Models';
import { PermissionsMenu } from './components/PermissionsMenu';
import { Collapsible } from './components/Collapsible';

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

    // Fun "ocean" of IF statements, hopefully covering every possible state of an installation!
    if(modStatus === null) {
        return <div className='container mainContainer fadeIn'>
            <h2>Checking Beat Saber installation...</h2>
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
        // Check if we can downgrade to a supported version
        const downgradeVersion = modStatus.core_mods
            .downgrade_versions
            .find(version => modStatus.core_mods!.supported_versions.includes(version));

        if(downgradeVersion === undefined) {
            return <NotSupported version={modStatus.app_info.version} device={device} quit={() => quit(undefined)} />
        }   else if(modStatus.app_info.loader_installed !== null) {
            // App is already patched, and we COULD in theory downgrade this version normally, but since it has been modified, the diffs will not work.
            // Therefore, they need to reinstall the latest version.
            return <IncompatibleAlreadyModded installedVersion={modStatus.app_info.version} device={device} quit={() => quit(undefined)}/>
        }   else    {
            return <PatchingMenu 
                modStatus={modStatus}
                onCompleted={status => setModStatus(status)}
                device={device}
                downgradingTo={downgradeVersion}
            />
        }

    }   else if(modStatus.app_info.loader_installed !== null)   {
        let loader = modStatus.app_info.loader_installed;
        if(loader === 'Scotland2') {
            return <>
                <div className='container mainContainer'>
                    <h1>App is modded</h1>
                    <p>Your Beat Saber install is modded, and its version is compatible with mods.</p>

                    <InstallStatus
                        modStatus={modStatus}
                        device={device}
                        onFixed={status => setModStatus(status)}
                    />
                    <h4>Not sure what to do next?</h4>
                    <NextSteps />
                </div>

                <ModManager modStatus={modStatus}
                    setMods={mods => setModStatus({ ...modStatus, installed_mods: mods })}
                    device={device}
                    gameVersion={modStatus.app_info.version}
                    quit={quit}
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
            downgradingTo={null} />
    }
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
    onCompleted: (newStatus: ModStatus) => void,
    downgradingTo: string | null
}

function PatchingMenu(props: PatchingMenuProps) {
    const [isPatching, setIsPatching] = useState(false);
    const [logEvents, addLogEvent] = useLog();
    const [patchingError, setPatchingError] = useState(null as string | null);
    const [manifestMod, setManifestMod] = useState({
        add_permissions: [],
        add_features: []
    } as ManifestMod);

    const { onCompleted, modStatus, device, downgradingTo } = props;
    if(!isPatching) {
        return <div className='container mainContainer'>
            {downgradingTo !== null && <DowngradeMessage toVersion={downgradingTo}/>}
            {downgradingTo === null && <VersionSupportedMessage version={modStatus.app_info!.version} />}
            
            <Collapsible title="Change App Permissions">
                <p>Certain mods may find it useful for the app to request microphone permissions or access to the headset cameras. Due to the privacy implications, you need to enable permissions here if you want them.</p>
                <PermissionsMenu manifestMod={manifestMod} setManifestMod={mod => setManifestMod(mod)} />
            </Collapsible>
            <h2 className='warning'>READ CAREFULLY</h2>
            <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
            <b>In addition, by modding the game you will lose access to both vanilla leaderboards and vanilla multiplayer.</b> (Modded leaderboards/servers are available.)

            <button className="modButton" onClick={async () => {
                setIsPatching(true);
                try {
                    onCompleted(await patchApp(device, modStatus, downgradingTo, manifestMod, false, addLogEvent));
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

function VersionSupportedMessage({ version }: { version: string }) {
    return <>
        <h1>Install Custom Songs</h1>
        <p>Your app has version {trimGameVersion(version)}, which is supported by mods!</p>
        <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
        Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>
    </>
}

function DowngradeMessage({ toVersion }: { toVersion: string }) {
    return <>
        <h1>Downgrade and set up mods</h1>
        <p>MBF has detected that your version of Beat Saber doesn't support mods!</p>

        <p>Fortunately for you, your version can be downgraded automatically to the latest moddable version: {trimGameVersion(toVersion)}</p>
        <p><span className='warning'><b>NOTE:</b></span> By downgrading, you will lose access to any DLCs or other content that is not present in version {trimGameVersion(toVersion)}. If you decide to stop using mods and reinstall vanilla Beat Saber, however, then you will get this content back.</p>
    </>
}

interface IncompatibleLoaderProps {
    loader: ModLoader,
    device: Adb,
    quit: () => void
}

function NotSupported({ version, quit, device }: { version: string, quit: () => void, device: Adb }) {
    return <div className='container mainContainer'>
        <h1>Unsupported Version</h1>
        <p className='warning'>Read this message in full before asking for help if needed!</p>

        <p>You have Beat Saber v{trimGameVersion(version)} installed, but this version has no support for mods!</p>
        <p>Normally, MBF would attempt to downgrade (un-update) your Beat Saber version to a version with mod support, but this is only possible if you have the latest version of Beat Saber installed.</p>
        <p>Please uninstall Beat Saber using the button below, then reinstall the latest version of Beat Saber using the Meta store.</p>

        <h4>Already have the latest version?</h4>
        <p>When a new Beat Saber version is added, the developer(s) of MBF must add the new version so you can downgrade. They're probably asleep right now, so give it a few hours.</p>


        <button onClick={async () => {
            await uninstallBeatSaber(device);
            quit();
        }}>Uninstall Beat Saber</button>
    </div>
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

function IncompatibleAlreadyModded({ device, quit, installedVersion }: { device: Adb,
    quit: () => void, installedVersion: string }) {
        return <div className='container mainContainer'>
            <h1>Incompatible Version Patched</h1>

            <p>Your Beat Saber app has a modloader installed, but the game version ({trimGameVersion(installedVersion)}) has no support for mods!</p>
            <p>To fix this, uninstall Beat Saber and reinstall the latest version. MBF can then downgrade this automatically to the latest moddable version.</p>

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