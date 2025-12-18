import { Adb, decodeUtf8 } from '@yume-chan/adb';
import { uninstallBeatSaber } from '../DeviceModder';
import { Children, ReactNode, useEffect, useRef, useState } from 'react';
import { fixPlayerData, patchApp, quickFix } from '../Agent';
import { toast } from 'react-toastify';
import { PermissionsMenu } from './PermissionsMenu';
import '../css/OptionsMenu.css'
import { Collapsible } from './Collapsible';
import { ModStatus } from '../Messages';
import { AndroidManifest } from '../AndroidManifest';
import { useSetError, wrapOperation } from '../SyncStore';
import { Log } from '../Logging';
import { Modal } from './Modal';
import { SplashScreenSelector } from './SplashScreenSelector';
import { useDeviceStore } from '../DeviceStore';
import { gameId } from '../game_info';
import { getLang } from '../localization/shared';

interface OptionsMenuProps {
    setModStatus: (status: ModStatus) => void,
    quit: (err: unknown | null) => void
    modStatus: ModStatus
    visible?: boolean
}
    
export function OptionsMenu({ quit, modStatus, setModStatus, visible }: OptionsMenuProps) {
    return <div className={`container mainContainer ${visible ? '' : 'hidden'}`} id="toolsContainer">
        <Collapsible title={getLang().optionMenuModTools} defaultOpen>
            <ModTools modStatus={modStatus} setModStatus={setModStatus} quit={() => quit(null)} />
        </Collapsible>
        <Collapsible title={getLang().optionMenuAdbLog} defaultOpen>
            <AdbLogger />
        </Collapsible>
        <Collapsible title={getLang().optionMenuChangePerm}>
            <RepatchMenu quit={quit} modStatus={modStatus}/>
        </Collapsible>
    </div>
}

function ModToolButton({ onClick, text, description }: { onClick: () => void, text: string|JSX.Element, description: string|JSX.Element }) {
    return <div>
      <div>
        <button onClick={onClick}>{text}</button>
      </div>
      <span>{description}</span>
    </div>
}

// Basic tools to do with managing the install, including a fix for a previously introduced bug.
function ModTools({ quit, modStatus, setModStatus }: {
    quit: () => void,
    modStatus: ModStatus,
    setModStatus: (status: ModStatus) => void}) {
    const { device } = useDeviceStore((store) => ({ device: store.device }));

    return (
      <div id="modTools">
        <ModToolButton
          text={getLang().optKillBeatSaber}
          description={getLang().optKillBeatSaberDesc}
          onClick={async () => {
            if (!device) return;

            const setError = useSetError(getLang().failedToKillBeatsaber);
            try {
                await device.subprocess.noneProtocol.spawnWait(`am force-stop ${gameId}`);
                toast.success(getLang().beatsaberKilled);
            }   catch(e) {
                setError(e);
            }
          }}
        />
        <ModToolButton
          text={getLang().optRestartBeatSaber}
          description={getLang().optRestartBeatSaberDesc}
          onClick={async () => {
            if (!device) return;

            const setError = useSetError(getLang().failedToKillBeatsaber);
            try {
              await device.subprocess.noneProtocol.spawnWait(`sh -c 'am force-stop ${gameId}; monkey -p com.beatgames.beatsaber -c android.intent.category.LAUNCHER 1'`);
              toast.success(getLang().beatsaberRestarted);
            } catch (e) {
              setError(e);
            }
          }}
        />
        <ModToolButton
          text={getLang().optReinstallCore}
          description={getLang().optReinstallCoreDesc}
          onClick={async () => {
            if (!device) return;

            await wrapOperation(
              getLang().reinstallOnlyCoreMods,
              getLang().failedToReinstallOnlyCoreMods,
              async () => {
                setModStatus(await quickFix(device, modStatus, true));
                toast.success(getLang().allNonCoreRemoved);
              }
            );
          }}
        />
        <ModToolButton
          text={getLang().optUninstallBeatsaber}
          description={getLang().optUninstallBeatsaberDesc}
          onClick={async () => {
            if (!device) return;

            const setError = useSetError(getLang().failedToUninstall);
            try {
              await uninstallBeatSaber(device);
              quit();
            } catch (e) {
              setError(e);
            }
          }}
        />
        <ModToolButton
          text={getLang().optFixPlayerData}
          description={getLang().optFixPlayerDataDesc}
          onClick={async () => {
            if (!device) return;
            const setError = useSetError(getLang().failedToFixPlayerData);
            try {
              if (await fixPlayerData(device)) {
                toast.success(getLang().optFixPlayerDataSuccess);
              } else {
                toast.error(getLang().optFixPlayerDataNoData);
              }
            } catch (e) {
              setError(e);
            }
          }}
        />
      </div>
    );
}

function RepatchMenu({ modStatus, quit }: {
    modStatus: ModStatus,
    quit: (err: unknown) => void
}
) {
    const { device, devicePreV51 } = useDeviceStore((store) => ({ 
        device: store.device,
        devicePreV51: store.devicePreV51
    }));

    let manifest = useRef(new AndroidManifest(modStatus.app_info!.manifest_xml));
    useEffect(() => {
        manifest.current.applyPatchingManifestMod(devicePreV51);
    }, []);
    const [splashScreen, setSplashScreen] = useState(null as File | null);

    return <>
        {getLang().changePermHintInOptionsMenu}
        <PermissionsMenu manifest={manifest.current} />
        <br/>
        <button onClick={async () => {
            if (!device) return;

            await wrapOperation(getLang().repatchingBeatSaber, getLang().failedToRepatch, async () => {
                // TODO: Right now we do not set the mod status back to the DeviceModder state for it.
                // This is fine at the moment since repatching does not update this state in any important way,
                // but would be a problem if repatching did update it!
                await patchApp(device, modStatus, null, manifest.current.toString(), true, false, false, splashScreen);
                toast.success(getLang().successfullyRepatched);
            })
        }}>{getLang().repatchGame}</button>

        <SplashScreenSelector selected={splashScreen}
            onSelected={nowSelected => setSplashScreen(nowSelected)} />
    </>
}

// Starts recording log messages from `adb logcat`. The promise returned will not complete until `getCancelled` returns a `true` value.
// Returns a blob containing the logcat messages recorded.
async function logcatToBlob(device: Adb, getCancelled: () => boolean): Promise<Blob> {
    Log.debug("Starting `logcat` process");

    // First clear the logcat buffer - we only want logs from events happening after the "start logcat" button is pressed.
    await device.subprocess.noneProtocol.spawnWait("logcat -c");
    
    const process = await device.subprocess.noneProtocol.spawn("logcat");
    let killed = false;

    Log.debug("Generating logs");
    const stdout = process.output.getReader();
    const logs = [];

    while(true) {
        const bytesRead = (await stdout.read()).value;
        if(bytesRead != null) {
            logs.push(decodeUtf8(bytesRead));
        }   else    {
            break;
        }

        // NB: It is vital that, after we kill logcat, we read any messages that have not yet been read
        // before returning. Otherwise, the unread messages cause the ADB implementation to hang on all future requests!
        if(getCancelled() && !killed) {
            Log.debug("Killing `logcat` process");
            await process.kill();
            killed = true;
        }
    }

    Log.debug("Providing blob of logs");
    return new Blob(logs, { type: 'text/plain' })
}

function AdbLogger() {
    const [logging, setLogging] = useState(false);
    const [logFile, setLogFile] = useState(null as Blob | null);
    const [waitingForLog, setWaitingForLog] = useState(false);
    const { device } = useDeviceStore((store)=> ({device: store.device}));

    useEffect(() => {
        if(!logging || !device) {
            return () => {};
        }

        // Begin gathering logs, making sure to remove the previous log file/blob
        setWaitingForLog(false);
        setLogFile(null);
        let cancelled = false;
        logcatToBlob(device, () => cancelled)
            .then(log => {
                setLogFile(log);
                setWaitingForLog(false);
            })
            .catch(e => Log.error("Failed to get ADB log " + e));
        
        // When the value of `logging` changes to false, use the cleanup function to tell the `log` function to stop getting logs as soon as it can.
        return () => {
            cancelled = true;
            setWaitingForLog(true);
        };
    }, [logging]);

    return <>
        {getLang().optionsMenuAdbLogHint}
        <p className="warning"></p>
        {!logging ? 
            <button onClick={async () => setLogging(true)}>{getLang().startLogging}</button> : 
            <button onClick={() => setLogging(false)}>{getLang().stopLogging}</button>}
            <br/>

        {waitingForLog && <p>{getLang().waitingForLog}</p>}
        {logFile !== null && <a href={URL.createObjectURL(logFile)} download={"logcat.log"}><button>{getLang().downloadLog}</button></a>}
    </>
}