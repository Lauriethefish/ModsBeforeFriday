import { Adb } from '@yume-chan/adb';
import { getDowngradedManifest, loadModStatus, patchApp, quickFix } from "./Agent";
import { ReactNode, useEffect, useState } from 'react';
import { ModLoader, ModStatus } from './Messages';
import './css/DeviceModder.css';
import { LogWindow, LogWindowControls } from './components/LogWindow';
import { ErrorModal, Modal } from './components/Modal';
import { ModManager } from './components/ModManager';
import { trimGameVersion } from './Models';
import { PermissionsMenu } from './components/PermissionsMenu';
import { SelectableList } from './components/SelectableList';
import { AndroidManifest } from './AndroidManifest';
import { Log } from './Logging';
import { wrapOperation } from './SyncStore';
import { OpenLogsButton } from './components/OpenLogsButton';
import { lte as semverLte } from 'semver';

interface DeviceModderProps {
    device: Adb,
    devicePreV51: boolean,
    // Quits back to the main menu, optionally giving an error that caused the quit.
    quit: (err: unknown | null) => void
}

export async function uninstallBeatSaber(device: Adb) {
    await device.subprocess.spawnAndWait("pm uninstall com.beatgames.beatsaber");
}

const isDeveloperUrl: boolean = new URLSearchParams(window.location.search).get("dev") === "true";

// Gets the available versions that have diffs to downgrade, and have core mod support
// Sorts them with the newest versions first.
export function GetSortedDowngradableVersions(modStatus: ModStatus): string[] | undefined {
    return modStatus.core_mods?.downgrade_versions
        .filter(version => modStatus.core_mods?.supported_versions.includes(version))
        .sort(CompareBeatSaberVersions)
}

export function CompareBeatSaberVersions(a: string, b: string): number {
    // Split each version into its segments, period separated,
    // e.g. 1.13.2 goes to 1, 13 and 2.
    // We make sure to remove the _ suffix
    const aSegments = a.split("_")[0].split(".");
    const bSegments = b.split("_")[0].split(".");

    // Iterate through the segments, from major to minor, until neither version has any more segments.
    for(let segment = 0; segment < Math.max(aSegments.length, bSegments.length); segment++) {
        // Default each segment to 0 if version A/B has terminated before this segment.
        let aSegment = 0;
        let bSegment = 0;
        if(segment < aSegments.length) {
            aSegment = Number(aSegments[segment]);
        }
        if(segment < bSegments.length) {
            bSegment = Number(bSegments[segment]);
        }

        if(aSegment > bSegment) {
            return -1;
        }   else if(aSegment < bSegment) {
            return 1;
        }
    }

    return 0;
}

export function DeviceModder(props: DeviceModderProps) {
    const [modStatus, setModStatus] = useState(null as ModStatus | null);
    const { device, quit, devicePreV51 } = props;

    useEffect(() => {
        loadModStatus(device)
            .then(loadedModStatus => setModStatus(loadedModStatus))
            .catch(err => quit(err));
    }, [device, quit]);

    // Fun "ocean" of IF statements, hopefully covering every possible state of an installation!
    if (modStatus === null) {
        return <div className='container mainContainer fadeIn'>
            <h2>Checking Beat Saber installation</h2>
            <span className="floatRight"><LogWindowControls/></span>
            <p>This might take a minute or so the first few times.</p>
            <LogWindow />
        </div>
    } else if (modStatus.app_info === null) {
        return <div className='container mainContainer'>
            <h1>Beat Saber is not installed</h1>
            <p>Please install Beat Saber from the store and then refresh the page.</p>
            <h3>Think you have Beat Saber installed?</h3>
            <p>Sometimes, it looks like Beat Saber is installed in your headset, when it actually isn't (a bug in the Meta software).</p>
            <p>This can be fixed by going to the main <b>Applications</b> menu inside your Quest, clicking the 3 dots next to Beat Saber, and clicking <b>Uninstall</b>. Finally, reinstall Beat Saber from the Meta store and refresh this page to try again.</p>
        </div>
    } else if (modStatus.core_mods === null) {
        return <div className='container mainContainer'>
            <OpenLogsButton />
            <h1>No internet</h1>
            <p>It seems as though <b>your Quest</b> has no internet connection.</p>
            <p>To mod Beat Saber, MBF needs to download files such as a mod loader and several essential mods.
                <br />This occurs on your Quest's connection. Please make sure that WiFi is enabled, then refresh the page.</p>
        </div>
    }  else if (!(modStatus.core_mods.supported_versions.includes(modStatus.app_info.version)) && !isDeveloperUrl) {
        // Check if we can downgrade to a supported version
        const downgradeVersions = GetSortedDowngradableVersions(modStatus);
        Log.debug("Available versions to downgrade: " + downgradeVersions);
        if(downgradeVersions === undefined || downgradeVersions.length === 0) {
            if(modStatus.core_mods.is_awaiting_diff) {
                return <NoDiffAvailable version={modStatus.app_info.version} />
            }   else    {
                return <NotSupported version={modStatus.app_info.version} device={device} quit={() => quit(undefined)} />
            }
        } else if (modStatus.app_info.loader_installed !== null) {
            // App is already patched, and we COULD in theory downgrade this version normally, but since it has been modified, the diffs will not work.
            // Therefore, they need to reinstall the latest version.
            return <IncompatibleAlreadyModded installedVersion={modStatus.app_info.version} device={device} quit={() => quit(undefined)} />
        } else if (!modStatus.app_info.obb_present) {
            // After we've checked (downgrade) version compatibility, next check if we're missing the OBB
            // We check this afterward so that, if the version is incorrect, the user is warned to reinstall *the correct version*.
            // Reinstalling will fix the OBB, and the OBB message doesn't mention the version
            return <NoObb device={device} quit={() => quit(undefined)}/>
        } else {
            return <PatchingMenu
                quit={quit}
                modStatus={modStatus}
                onCompleted={status => setModStatus(status)}
                device={device}
                devicePreV51={devicePreV51}
                initialDowngradingTo={downgradeVersions[0]}
            />
        }

    }   else if (!modStatus.app_info.obb_present) { // Before allowing modding the installed version without downgrading, check for missing OBB.
        return <NoObb device={device} quit={() => quit(undefined)}/>
    }   else if (modStatus.app_info.loader_installed !== null) {
        let loader = modStatus.app_info.loader_installed;
        if(loader === 'Scotland2') {
            return <ValidModLoaderMenu device={device} modStatus={modStatus} setModStatus={setModStatus} quit={() => quit(null)}/>
        }   else    {
            return <IncompatibleLoader device={device} loader={loader} quit={() => quit(null)} />
        }
    } else {
        return <PatchingMenu
            quit={quit}
            device={device}
            devicePreV51={devicePreV51}
            modStatus={modStatus}
            onCompleted={modStatus => setModStatus(modStatus)}
            initialDowngradingTo={null} />
    }
}

function NoObb({ device, quit }: { device: Adb, quit: () => void }) {
     return <div className="container mainContainer">
        <h1>OBB not present</h1>
        <p>MBF has detected that the OBB file, which contains asset files required for Beat Saber to load, is not present in the installation.</p>
        <p>This means your installation is corrupt. You will need to uninstall Beat Saber with the button below, and reinstall the latest version from the Meta store.</p>
        <button onClick={async () => {
            await uninstallBeatSaber(device);
            quit();
        }}>Uninstall Beat Saber</button>
     </div>
}

function ValidModLoaderMenu({ device, modStatus, setModStatus, quit }: { device: Adb,
    modStatus: ModStatus,
    setModStatus: (status: ModStatus) => void
    quit: () => void}) {

    return <>
        <div className='container mainContainer'>
            <OpenLogsButton />
            <h1>App is modded</h1>
            <UpdateInfo modStatus={modStatus} device={device} quit={quit}/>

            {isDeveloperUrl ? <>
                <p className="warning">Core mod functionality is disabled.</p>
            </> : <>
                <InstallStatus
                        modStatus={modStatus}
                        device={device}
                        onFixed={status => setModStatus(status)}/>
                <h4>Not sure what to do next?</h4>
                <NextSteps />
            </>}
        </div>

        <ModManager modStatus={modStatus}
            setMods={mods => {
                // When changing mod statuses, the core mods are not fetched by the backend, so it doesn't know which mods are core
                // Therefore, any previously core mods need to have this status copied over to the newly set mods
                modStatus.installed_mods.filter(existing => existing.is_core)
                    .forEach(existing => {
                        let new_mod = mods.find(mod => mod.id === existing.id);
                        if(new_mod) {
                            new_mod.is_core = true;
                        }
                    })

                setModStatus({ ...modStatus, installed_mods: mods });
            }}
            setModStatus={status => setModStatus(status)}
            device={device}
            gameVersion={modStatus.app_info!.version}
            quit={quit}
        />
    </>
}

interface InstallStatusProps {
    modStatus: ModStatus
    onFixed: (newStatus: ModStatus) => void,
    device: Adb
}

function InstallStatus(props: InstallStatusProps) {
    const { modStatus, onFixed, device } = props;

    const modloaderStatus = modStatus.modloader_install_status;
    const coreModStatus = modStatus.core_mods!.core_mod_install_status;

    if (modloaderStatus === "Ready" && coreModStatus === "Ready") {
        return <p>Everything should be ready to go! &#9989;</p>
    } else {
        return <div>
            <h3 className="warning">Problems found with your install:</h3>
            <p>These can be easily fixed by clicking the button below.</p>
            <ul>
                {modloaderStatus === "Missing" &&
                    <li>Modloader not found &#10060;</li>}
                {modloaderStatus === "NeedUpdate" &&
                    <li>Modloader has an available update</li>}
                {coreModStatus === "Missing" &&
                    <li>Not all the core mods are installed &#10060;</li>}
                {coreModStatus === "NeedUpdate" && 
                    <li>Core mod updates need to be installed.</li>}
            </ul>
            <button onClick={async () => {
                wrapOperation("Fixing issues", "Failed to fix install", async () =>
                    onFixed(await quickFix(device, modStatus, false)));
            }}>Fix issues</button>
        </div>
    }
}

function UpdateInfo({ modStatus, device, quit }: { modStatus: ModStatus, device: Adb, quit: () => void }) {
    const sortedModdableVersions = modStatus.core_mods!.supported_versions.sort(CompareBeatSaberVersions);
    const newerUpdateExists = modStatus.app_info?.version !== sortedModdableVersions[0];

    const [updateWindowOpen, setUpdateWindowOpen] = useState(false);

    return <>
        <p>Your Beat Saber install is modded, and its version is compatible with mods.</p>
        {newerUpdateExists && <p>&#10071; &#65039;&#10071; &#65039; However, an updated moddable version is available! <ClickableLink onClick={() => setUpdateWindowOpen(true)}>Click here to update</ClickableLink></p>}

        <Modal isVisible={updateWindowOpen}>
            <h2>Update Beat Saber</h2>
            <p>To update to the latest moddable version, simply:</p>
            <ol>
                <li>Uninstall Beat Saber with the button below.</li>
                <li>Reinstall Beat Saber in your headset.</li>
                <li>Open back up MBF to mod the version you just installed.</li>
            </ol>
            <button onClick={async () => {
                await uninstallBeatSaber(device);
                quit();
            }}>Uninstall Beat Saber</button>
            <button onClick={() => setUpdateWindowOpen(false)} className="discreetButton">Cancel</button>
            <br/><br/>
            <h3>What about my maps/mods/scores/qosmetics?</h3>
            <ul>
                <li><em>Maps and scores will remain safe</em> as they are held in a separate folder to the game files, so will not be deleted when you uninstall.</li>
                <li>Qosmetics will not be deleted, however you can only use them if the qosmetics mods are available for the new version. You can always move back to your current version later if you miss them.</li>
                <li><em>All currently installed mods will be removed.</em> (the new core mods for the updated version will be installed automatically) You can reinstall them once you have updated (if they are available for the newer version)</li>
            </ul>
        </Modal>
    </>
}

interface PatchingMenuProps {
    modStatus: ModStatus
    device: Adb,
    devicePreV51: boolean,
    onCompleted: (newStatus: ModStatus) => void,
    initialDowngradingTo: string | null,
    quit: (err: unknown) => void
}

function PatchingMenu(props: PatchingMenuProps) {
    const [isPatching, setIsPatching] = useState(false);
    const [patchingError, setPatchingError] = useState(null as string | null);
    const [selectingPerms, setSelectingPerms] = useState(false);
    const [versionSelectOpen, setVersionSelectOpen] = useState(false);
    const [versionOverridden, setVersionOverridden] = useState(false);

    const { onCompleted, modStatus, device, initialDowngradingTo, devicePreV51 } = props;
    const [downgradingTo, setDowngradingTo] = useState(initialDowngradingTo);
    const downgradeChoices = GetSortedDowngradableVersions(modStatus)!
    .filter(version => version != initialDowngradingTo);
    
    const [manifest, setManifest] = useState(null as null | AndroidManifest); 
    manifest?.applyPatchingManifestMod();
    
    useEffect(() => {
        if(downgradingTo === null) {
            setManifest(new AndroidManifest(props.modStatus.app_info!.manifest_xml));
        }   else    {
            getDowngradedManifest(device, downgradingTo)
            .then(manifest_xml => setManifest(new AndroidManifest(manifest_xml)))
            .catch(error => {
                // TODO: Perhaps revert to "not downgrading" if this error comes up (but only if the latest version is moddable)
                // This is low priority as this error message should only show up very rarely - there is already a previous check for internet access.
                Log.error("Failed to fetch older manifest: " + error);
                props.quit("Failed to fetch AndroidManifest.xml for the selected downgrade version. Did your quest lose its internet connection suddenly?");
            });
        }
    }, [downgradingTo]);

    if(manifest === null) {
        return <div className='container mainContainer'>
            <h2>Loading downgraded APK manifest</h2>
            <p>This should only take a few seconds.</p>
        </div>
    } else if(!isPatching) {
        return <div className='container mainContainer'>
            <OpenLogsButton />

            {downgradingTo !== null && <DowngradeMessage
                toVersion={downgradingTo}
                wasUserSelected={versionOverridden}
                requestedVersionChange={() => setVersionSelectOpen(true)}
                canChooseAnotherVersion={downgradeChoices.length > 0}
                requestedResetToDefault={() => {
                setDowngradingTo(initialDowngradingTo);
                setVersionOverridden(false);
            }} />}

            {downgradingTo === null && <VersionSupportedMessage
                version={modStatus.app_info!.version}
                requestedVersionChange={() => setVersionSelectOpen(true)}
                canChooseAnotherVersion={downgradeChoices.length > 0}/>}
            <PatchingAdvancedOptions
                isVisible={versionSelectOpen && downgradeChoices.length > 0}
                downgradeVersions={downgradeChoices}
                onConfirm={version => {
                    if(version !== null) {
                        setDowngradingTo(version);
                        setManifest(null);
                        setVersionOverridden(true);
                    }
                    setVersionSelectOpen(false);
                }} />
            
            <h2 className='warning'>READ CAREFULLY</h2>
            <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
            <div>
                <button className="discreetButton" id="permissionsButton" onClick={() => setSelectingPerms(true)}>Permissions</button>
                <button className="largeCenteredButton" onClick={async () => {
                    setIsPatching(true);
                    try {
                        onCompleted(await patchApp(device, modStatus, downgradingTo, manifest.toString(), false, isDeveloperUrl, devicePreV51, null));
                    } catch (e) {
                        setPatchingError(String(e));
                        setIsPatching(false);
                    }
                }}>Mod the app</button>
            </div>

            <ErrorModal
                isVisible={patchingError != null}
                title={"Failed to install mods"}
                description={'An error occured while patching ' + patchingError}
                onClose={() => setPatchingError(null)} />

            <Modal isVisible={selectingPerms}>
                <h2>Change Permissions</h2>
                <p>Certain mods require particular Android permissions to be set on the Beat Saber app in order to work correctly.</p>
                <p>(You can change these permissions later, so don't worry about enabling them all now unless you know which ones you need.)</p>
                <PermissionsMenu manifest={manifest} />
                <button className="largeCenteredButton" onClick={() => setSelectingPerms(false)}>Confirm permissions</button>
            </Modal>

        </div>
    } else {
        return <div className='container mainContainer'>
            <h1>App is being patched</h1>
            <p>This should only take a few minutes, but could take much, much longer if your internet connection is slow.</p>
            <span className="floatRight"><LogWindowControls/></span>
            <p className='warning'>You must not disconnect your device during this process.</p>
            <LogWindow />
        </div>
    }
}

function PatchingAdvancedOptions({ downgradeVersions, onConfirm, isVisible }:
    {
        downgradeVersions: string[],
        isVisible: boolean,
        onConfirm: (version: string | null) => void
    }) {

    const [selected, setSelected] = useState(null as string | null);

    return <Modal isVisible={isVisible}>
        <h2>Choose a different game version</h2>
        <p>Using this menu, you can make MBF downgrade to a version other than the latest moddable version.</p>
        <p>This is not recommended, and you should only do it if there are specific mods you wish to use that aren't available on the latest version.</p>
        <p><b>Please note that MBF is not capable of modding Beat Saber 1.28 or lower.</b></p>
        <p>Click a version then confirm the downgrade:</p>
        <SelectableList options={downgradeVersions} choiceSelected={choice => setSelected(choice)} />
        <br/>
        <button onClick={() => {
            setSelected(null);
            onConfirm(selected);
        }}>{selected === null ? <>Use latest moddable</> : <>Confirm downgrade</>}</button>
    </Modal>
}

function ClickableLink({ onClick, children }: { onClick: () => void, children: ReactNode }) {
    return <a className="clickableLink" onClick={onClick}>{children}</a>
}

function VersionSupportedMessage({ version, requestedVersionChange, canChooseAnotherVersion }: { version: string, requestedVersionChange: () => void, canChooseAnotherVersion: boolean }) {
    return <>
        <h1>Install Custom Songs</h1>
        {isDeveloperUrl ?
            <p className="warning">Mod development mode engaged: bypassing version check.
            This will not help you unless you are a mod developer!</p> : <>
            <p>Your app has version {trimGameVersion(version)}, which is supported by mods! {canChooseAnotherVersion && <ClickableLink onClick={requestedVersionChange}>(choose another version)</ClickableLink>}</p>
            <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
            Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>
        </>}
    </>
}

function DowngradeMessage({ toVersion, wasUserSelected, requestedVersionChange, requestedResetToDefault, canChooseAnotherVersion }: { toVersion: string, wasUserSelected: boolean,
    requestedVersionChange: () => void,
    requestedResetToDefault: () => void,
    canChooseAnotherVersion: boolean }) {
    return <>
        <h1>Downgrade and set up mods</h1>
        {wasUserSelected ? (<><p>You have decided to downgrade to a version older than the latest moddable version. <b>Only do this if you know why you want to!</b> <ClickableLink onClick={requestedResetToDefault}>(reverse decision)</ClickableLink></p></>)
        : <>
            <p>MBF has detected that your version of Beat Saber doesn't support mods!</p>
            <p>Fortunately for you, your version can be downgraded automatically to the latest moddable version: {trimGameVersion(toVersion)} {canChooseAnotherVersion && <ClickableLink onClick={requestedVersionChange}>(choose another version)</ClickableLink>}</p>
        </>}
        <p><span className='warning'><b>NOTE:</b></span> By downgrading, you will lose access to any DLC or other content that is not present in version {trimGameVersion(toVersion)}. If you decide to stop using mods and reinstall vanilla Beat Saber, however, then you will get this content back.</p>
    </>
}

interface IncompatibleLoaderProps {
    loader: ModLoader,
    device: Adb,
    quit: () => void
}

function NotSupported({ version, quit, device }: { version: string, quit: () => void, device: Adb }) {
    const isLegacy = isVersionLegacy(version);

    return <div className='container mainContainer'>
        <h1>Unsupported Version</h1>
        <p className='warning'>Read this message in full before asking for help if needed!</p>

        {/* Some legacy versions can be modded but MBF does not support anything on the old Unity version*/}
        <p>You have Beat Saber v{trimGameVersion(version)} installed, but this version has no support for {isLegacy ? "modding with MBF" : "mods"}!</p>
        {isLegacy && <>
            <p>While your version might work with other modding tools, it is <b>very strongly recommended</b> that you uninstall Beat Saber and update
        to the latest moddable version.</p>

            <p className="warning">Modding with versions 1.28.0 or below is <em>no longer supported by BSMG</em> - it's an ancient version and nobody should be using it anymore. Please, please, <em>PLEASE</em> update to the latest version of the game.</p>
        </>}

        {!isLegacy && <>
            <p>Normally, MBF would attempt to downgrade (un-update) your Beat Saber version to a version with mod support, but this is only possible if you have the latest version of Beat Saber installed.</p>
            <p>Please uninstall Beat Saber using the button below, then reinstall the latest version of Beat Saber using the Meta store.</p>
        </>}

        <button onClick={async () => {
            await uninstallBeatSaber(device);
            quit();
        }}>Uninstall Beat Saber</button>
    </div>
}


// Works out if the passed Beat Saber version is legacy (QuestLoader - not MBF supported), i.e. v1.28.0 or less.
function isVersionLegacy(version: string): boolean {
    const sem_version = version.split('_')[0];
    return semverLte(sem_version, "1.28.0");
}

function NoDiffAvailable({ version }: { version: string }) {
    return <div className="container mainContainer">
        <h1>Awaiting patch generation</h1>
        <p className='warning'>You must READ this message IN FULL.</p>

        <p>You have Beat Saber v{trimGameVersion(version)}, which has no support for mods.</p>
        <p>MBF is designed to downgrade (un-update) your Beat Saber version to a version with mod support <b>but the necessary patches have not yet been generated,</b> as a Beat Saber update has only just been released.</p>
        <p>Patch generation needs manual input and <b>will happen as soon as the author of MBF is available</b>, which will take from 30 minutes to 24 hours.</p>
        <p><b>PLEASE WAIT in the meanwhile.</b> You can refresh this page and reconnect to your Quest to check if the patch has been generated.</p>
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

function IncompatibleAlreadyModded({ device, quit, installedVersion }: {
    device: Adb,
    quit: () => void, installedVersion: string
}) {
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