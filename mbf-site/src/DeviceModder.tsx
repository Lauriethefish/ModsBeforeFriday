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
import { useDeviceStore } from './DeviceStore';
import { gameId } from './game_info';
import { getLang } from './localization/shared';

interface DeviceModderProps {
    device: Adb,
    devicePreV51: boolean,
    // Quits back to the main menu, optionally giving an error that caused the quit.
    quit: (err: unknown | null) => void
}

export async function uninstallBeatSaber(device: Adb) {
    await device.subprocess.noneProtocol.spawnWait(`pm uninstall ${gameId}`);
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
    const { quit } = props;
    const { device } = useDeviceStore((state) => ({ device: state.device }));

    useEffect(() => {
        if (!device) { return; } // If the device is not set, do not attempt to load mod status.
        loadModStatus(device)
            .then(loadedModStatus => setModStatus(loadedModStatus))
            .catch(err => quit(err));
    }, [device]);

    // Fun "ocean" of IF statements, hopefully covering every possible state of an installation!
    if (modStatus === null) {
        return <div className='container mainContainer fadeIn'>
            <h2>{getLang().checkInstall}</h2>
            <span className="floatRight"><LogWindowControls/></span>
            <p>{getLang().mightTakeFewTimes}</p>
            <LogWindow />
        </div>
    } else if (modStatus.app_info === null) {
        return <div className='container mainContainer'>
            { getLang().notInstalled }
        </div>
    } else if (modStatus.core_mods === null) {
        return <div className='container mainContainer'>
            <OpenLogsButton />
            { getLang().noInternet }
        </div>
    }  else if (!(modStatus.core_mods.supported_versions.includes(modStatus.app_info.version)) && !isDeveloperUrl) {
        // Check if we can downgrade to a supported version
        const downgradeVersions = GetSortedDowngradableVersions(modStatus);
        Log.debug("Available versions to downgrade: " + downgradeVersions);
        if(downgradeVersions === undefined || downgradeVersions.length === 0) {
            if(modStatus.core_mods.is_awaiting_diff) {
                return <NoDiffAvailable version={modStatus.app_info.version} />
            }   else    {
                return <NotSupported version={modStatus.app_info.version} quit={() => quit(undefined)} />
            }
        } else if (modStatus.app_info.loader_installed !== null) {
            // App is already patched, and we COULD in theory downgrade this version normally, but since it has been modified, the diffs will not work.
            // Therefore, they need to reinstall the latest version.
            return <IncompatibleAlreadyModded installedVersion={modStatus.app_info.version} quit={() => quit(undefined)} />
        } else if (!modStatus.app_info.obb_present) {
            // After we've checked (downgrade) version compatibility, next check if we're missing the OBB
            // We check this afterward so that, if the version is incorrect, the user is warned to reinstall *the correct version*.
            // Reinstalling will fix the OBB, and the OBB message doesn't mention the version
            return <NoObb quit={() => quit(undefined)}/>
        } else {
            return <PatchingMenu
                quit={quit}
                modStatus={modStatus}
                onCompleted={status => setModStatus(status)}
                initialDowngradingTo={downgradeVersions[0]}
            />
        }

    }   else if (!modStatus.app_info.obb_present) { // Before allowing modding the installed version without downgrading, check for missing OBB.
        return <NoObb quit={() => quit(undefined)}/>
    }   else if (modStatus.app_info.loader_installed !== null) {
        let loader = modStatus.app_info.loader_installed;
        if(loader === 'Scotland2') {
            return <ValidModLoaderMenu modStatus={modStatus} setModStatus={setModStatus} quit={() => quit(null)}/>
        }   else    {
            return <IncompatibleLoader loader={loader} quit={() => quit(null)} />
        }
    } else {
        return <PatchingMenu
            quit={quit}
            modStatus={modStatus}
            onCompleted={modStatus => setModStatus(modStatus)}
            initialDowngradingTo={null} />
    }
}

function NoObb({ quit }: { quit: () => void }) {
    const { device } = useDeviceStore((state) => ({ device: state.device }));

     return <div className="container mainContainer">
        {getLang().obbNotPresent}
        <button onClick={async () => {
            if (!device) return;

            await uninstallBeatSaber(device);
            quit();
        }}>{getLang().uninstallBeatSaber}</button>
     </div>
}

function ValidModLoaderMenu({ modStatus, setModStatus, quit }: {
    modStatus: ModStatus,
    setModStatus: (status: ModStatus) => void
    quit: () => void}) {
    const { device } = useDeviceStore((state) => ({ device: state.device }));

    return <>
        <div className='container mainContainer'>
            <OpenLogsButton />
            <h1>{getLang().appIsModded}</h1>
            <UpdateInfo modStatus={modStatus} quit={quit}/>

            {isDeveloperUrl ? <>
                <p className="warning">{getLang().coreModDisabled}</p>
            </> : <>
                <InstallStatus
                        modStatus={modStatus}
                        onFixed={status => setModStatus(status)}/>
                <h4>{getLang().notSureNext}</h4>
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
            gameVersion={modStatus.app_info!.version}
            quit={quit}
        />
    </>
}

interface InstallStatusProps {
    modStatus: ModStatus
    onFixed: (newStatus: ModStatus) => void
}

function InstallStatus(props: InstallStatusProps) {
    const { modStatus, onFixed } = props;

    const modloaderStatus = modStatus.modloader_install_status;
    const coreModStatus = modStatus.core_mods!.core_mod_install_status;
    const { device } = useDeviceStore((state) => ({ device: state.device }));
    
    if (modloaderStatus === "Ready" && coreModStatus === "Ready") {
        return <p>{getLang().everythingReady} &#9989;</p>
    } else {
        return <div>
            <h3 className="warning">{getLang().problemFound}</h3>
            <p>{getLang().problemCanFix}</p>
            <ul>
                {modloaderStatus === "Missing" &&
                    <li>{getLang().modloaderNotFound} &#10060;</li>}
                {modloaderStatus === "NeedUpdate" &&
                    <li>{getLang().modloaderNeedUpdate}</li>}
                {coreModStatus === "Missing" &&
                    <li>{getLang().coremodsMissing} &#10060;</li>}
                {coreModStatus === "NeedUpdate" && 
                    <li>{getLang().coreModsNeedUpdate}</li>}
            </ul>
            <button onClick={async () => {
                if (!device) return;

                wrapOperation("Fixing issues", "Failed to fix install", async () =>
                    onFixed(await quickFix(device, modStatus, false)));
            }}>{getLang().fixIssue}</button>
        </div>
    }
}

function UpdateInfo({ modStatus, quit }: { modStatus: ModStatus, quit: () => void }) {
    const sortedModdableVersions = modStatus.core_mods!.supported_versions.sort(CompareBeatSaberVersions);
    const newerUpdateExists = modStatus.app_info?.version !== sortedModdableVersions[0];
    const { device } = useDeviceStore((state) => ({ device: state.device }));

    const [updateWindowOpen, setUpdateWindowOpen] = useState(false);
    
    return <>
        <p>{ getLang().modCompatable }</p>
        {newerUpdateExists && <p>&#10071; &#65039;&#10071; &#65039; { getLang().modUpdateAvaliable }<ClickableLink onClick={() => setUpdateWindowOpen(true)}>{getLang().clickHereToUpdate}</ClickableLink></p>}

        <Modal isVisible={updateWindowOpen}>
            {getLang().updateBeatSaberHint}
            <button onClick={async () => {
                if (!device) return;
                await uninstallBeatSaber(device);
                quit();
            }}>{getLang().uninstallBeatSaber}</button>
            <button onClick={() => setUpdateWindowOpen(false)} className="discreetButton">{getLang().cancel}</button>
            <br/><br/>
            {getLang().uninstallAboutMapThings}
        </Modal>
    </>
}

interface PatchingMenuProps {
    modStatus: ModStatus,
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

    const { onCompleted, modStatus, initialDowngradingTo } = props;
    const { device, devicePreV51 } = useDeviceStore((state) => ({ device: state.device, devicePreV51: state.devicePreV51 }));
    const [downgradingTo, setDowngradingTo] = useState(initialDowngradingTo);
    const downgradeChoices = GetSortedDowngradableVersions(modStatus)!
    .filter(version => version != initialDowngradingTo);
    
    const [manifest, setManifest] = useState(null as null | AndroidManifest); 
    manifest?.applyPatchingManifestMod(devicePreV51);
    
    useEffect(() => {
        if (!device) return;

        if(downgradingTo === null) {
            setManifest(new AndroidManifest(props.modStatus.app_info!.manifest_xml));
        }   else    {
            getDowngradedManifest(device, downgradingTo)
            .then(manifest_xml => setManifest(new AndroidManifest(manifest_xml)))
            .catch(error => {
                // TODO: Perhaps revert to "not downgrading" if this error comes up (but only if the latest version is moddable)
                // This is low priority as this error message should only show up very rarely - there is already a previous check for internet access.
                Log.error("Failed to fetch older manifest: " + error);
                props.quit(getLang().failedToFetchManifestHint);
            });
        }
    }, [downgradingTo]);

    if(manifest === null) {
        return <div className='container mainContainer'>
            {getLang().loadingDowngradedApk}
        </div>
    } else if(!isPatching) {
        return <div className='container mainContainer'>
            <OpenLogsButton />

            {downgradingTo !== null && <DowngradeMessage
                toVersion={downgradingTo}
                wasUserSelected={versionOverridden}
                requestedVersionChange={() => setVersionSelectOpen(true)}
                canChooseAnotherVersion={downgradeChoices.length > 0}
                devicePreV51={devicePreV51}
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
            {getLang().modWarning}
            <div>
                <button className="discreetButton" id="permissionsButton" onClick={() => setSelectingPerms(true)}>{getLang().permissions}</button>
                <button disabled={!device} className="largeCenteredButton" onClick={async () => {
                    if (!device) return;

                    setIsPatching(true);
                    try {
                        onCompleted(await patchApp(device, modStatus, downgradingTo, manifest.toString(), false, isDeveloperUrl, devicePreV51, null));
                    } catch (e) {
                        setPatchingError(String(e));
                        setIsPatching(false);
                    }
                }}>{getLang().modTheApp}</button>
            </div>

            <ErrorModal
                isVisible={patchingError != null}
                title={"Failed to install mods"}
                description={'An error occured while patching ' + patchingError}
                onClose={() => setPatchingError(null)} />

            <Modal isVisible={selectingPerms}>
                {getLang().changePermissionHint}
                <PermissionsMenu manifest={manifest} />
                <button className="largeCenteredButton" onClick={() => setSelectingPerms(false)}>{getLang().confirmPermission}</button>
            </Modal>

        </div>
    } else {
        return <div className='container mainContainer'>
            {getLang().appPatchedHint}
            <span className="floatRight"><LogWindowControls/></span>
            <p className='warning'>{getLang().dontDisconnectDeviceHint}</p>
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
        {getLang().chooseDifferentGameVersionHint}
        <SelectableList options={downgradeVersions} choiceSelected={choice => setSelected(choice)} />
        <br/>
        <button onClick={() => {
            setSelected(null);
            onConfirm(selected);
        }}>{selected === null ? <>{getLang().useLatestModdable}</> : <>{getLang().confirmDowngrade}</>}</button>
    </Modal>
}

function ClickableLink({ onClick, children }: { onClick: () => void, children: ReactNode }) {
    return <a className="clickableLink" onClick={onClick}>{children}</a>
}

function VersionSupportedMessage({ version, requestedVersionChange, canChooseAnotherVersion }: { version: string, requestedVersionChange: () => void, canChooseAnotherVersion: boolean }) {
    return <>
        <h1>{getLang().versionSupportedMessageTitle}</h1>
        {isDeveloperUrl ?
            <p className="warning">{getLang().modDevelopmentWarn}</p> : <>
            <p>{getLang().versionSupportedHint(trimGameVersion(version))} {canChooseAnotherVersion && <ClickableLink onClick={requestedVersionChange}>{getLang().chooseAnotherVersion}</ClickableLink>}</p>
            {getLang().versionSupportedInstallEssentialMods}
        </>}
    </>
}

function DowngradeMessage({ toVersion,
    wasUserSelected,
    requestedVersionChange,
    devicePreV51,
    requestedResetToDefault,
    canChooseAnotherVersion }: { toVersion: string, wasUserSelected: boolean,
    requestedVersionChange: () => void,
    requestedResetToDefault: () => void,
    devicePreV51: boolean,
    canChooseAnotherVersion: boolean }) {
    return <>
        <h1>{devicePreV51 ? getLang().updateAndSetupMods : getLang().downgradeAndSetupMods}</h1>
        {wasUserSelected ? (<><p>{getLang().olderThanLatestModdableHint} <ClickableLink onClick={requestedResetToDefault}>{getLang().reverseDecision}</ClickableLink></p></>)
        : devicePreV51 ? (<>
            <p>{getLang().quest1ModHint(trimGameVersion(toVersion))}</p>
        </>) : <>
                <p>{getLang().doesntSupportMods}</p>
                <p>{getLang().canDowngrateToVersion(trimGameVersion(toVersion))} {canChooseAnotherVersion && <ClickableLink onClick={requestedVersionChange}>{getLang().chooseAnotherVersion}</ClickableLink>}</p>
            </>}
        </>
}

interface IncompatibleLoaderProps {
    loader: ModLoader,
    quit: () => void
}

function NotSupported({ version, quit }: { version: string, quit: () => void }) {
    const { device } = useDeviceStore((state) => ({ device: state.device }));
    const isLegacy = isVersionLegacy(version);

    return <div className='container mainContainer'>
        <h1>{getLang().unsupportedVersion}</h1>
        <p className='warning'>{getLang().readThisMessage}</p>

        {/* Some legacy versions can be modded but MBF does not support anything on the old Unity version*/}
        <p>{getLang().notSupportedModsText(trimGameVersion(version), isLegacy)}</p>
        {isLegacy && <>
            {getLang().legacyUpdateRecommand}
        </>}

        {!isLegacy && <>
            {getLang().normallyUpdateRecommand}
        </>}

        <button onClick={async () => {
            if (!device) return;

            await uninstallBeatSaber(device);
            quit();
        }}>{getLang().uninstallBeatSaber}</button>
    </div>
}


// Works out if the passed Beat Saber version is legacy (QuestLoader - not MBF supported), i.e. v1.28.0 or less.
function isVersionLegacy(version: string): boolean {
    const sem_version = version.split('_')[0];
    return semverLte(sem_version, "1.28.0");
}

function NoDiffAvailable({ version }: { version: string }) {
    return <div className="container mainContainer">
        <h1>{getLang().awaitingPatchGeneration}</h1>
        <p className='warning'>{getLang().mustReadMessageFull}</p>

        {getLang().noDiffMessageBody(trimGameVersion(version))}
    </div>
}

function IncompatibleLoader(props: IncompatibleLoaderProps) {
    const { loader, quit } = props;
    const { device } = useDeviceStore((state) => ({ device: state.device }));

    return <div className='container mainContainer'>
        {getLang().incompatableModLoader(loader)}

        <button onClick={async () => {
            if (!device) return;

            await uninstallBeatSaber(device);
            quit();
        }}>{getLang().uninstallBeatSaber}</button>
    </div>
}

function IncompatibleAlreadyModded({ quit, installedVersion }: {
    quit: () => void, installedVersion: string
}) {
    const { device } = useDeviceStore((state) => ({ device: state.device }));

    return <div className='container mainContainer'>
        {getLang().incompatableVersionPatched(trimGameVersion(installedVersion))}
        <button onClick={async () => {
            if (!device) return;
            
            await uninstallBeatSaber(device);
            quit();
        }}>{getLang().uninstallBeatSaber}</button>
    </div>
}

function NextSteps() {
    return <>{getLang().nextSteps}</>
}