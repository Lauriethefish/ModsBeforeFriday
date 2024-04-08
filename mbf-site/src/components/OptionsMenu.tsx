import { Adb, decodeUtf8 } from '@yume-chan/adb';
import { uninstallBeatSaber } from '../DeviceModder';
import { LogEventSink, quickFix } from '../Agent';
import { useEffect, useState } from 'react';


export function OptionsMenu({ device, quit, setError }: {
    device: Adb,
    quit: () => void,
    setError: (err: string) => void}) {
    return <>
        <div className="container mainContainer" id="toolsContainer">
            <h2>Mod Tools</h2>
            <br />
            <button onClick={async () => {
                try {
                    await uninstallBeatSaber(device);
                    quit();
                }   catch(e)   {
                    setError("Failed to uninstall Beat Saber " + e)
                }
            }}>Uninstall Beat Saber</button>

            <AdbLogger device={device}/>
        </div>
    </>
}

// Starts recording log messages from `adb logcat`. The promise returned will not complete until `getCancelled` returns a `true` value.
// Returns a blob containing the logcat messages recorded.
async function logcatToBlob(device: Adb, getCancelled: () => boolean): Promise<Blob> {
    console.log("Starting `logcat` process");

    // First clear the logcat buffer - we only want logs from events happening after the "start logcat" button is pressed.
    await device.subprocess.spawnAndWait("logcat -c");
    
    const process = await device.subprocess.spawn("logcat");
    let killed = false;

    console.log("Generating logs");
    const stdout = process.stdout.getReader();
    const logs = [];

    while(true) {
        const bytesRead = (await stdout.read()).value;
        if(bytesRead != null) {
            logs.push(decodeUtf8(bytesRead));
        }   else    {
            break;
        }

        // NB: It is vital that, after we kill logcat, we read any messages that have not yet been read
        // before returning. Otherwise, the unread messages causes the ADB implementation to hang on all future requests!
        if(getCancelled() && !killed) {
            console.log("Killing `logcat` process");
            await process.kill();
            killed = true;
        }
    }

    console.log("Providing blob of logs");
    return new Blob(logs, { type: 'text/plain' })
}

function AdbLogger({ device }: { device: Adb }) {
    const [logging, setLogging] = useState(false);
    const [logFile, setLogFile] = useState(null as Blob | null);
    const [waitingForLog, setWaitingForLog] = useState(false);

    useEffect(() => {
        if(!logging) {
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
            .catch(e => console.error("Failed to get ADB log " + e));
        
        // When the value of `logging` changes to false, use the cleanup function to tell the `log` function to stop getting logs as soon as it can.
        return () => {
            cancelled = true;
            setWaitingForLog(true);
        };
    }, [logging]);

    return <>
        <h2>ADB Log</h2>
        <p>This feature allows you to get a log of what's going on inside your Quest, useful for modders to fix bugs with their mods.</p>
        <p>Click the button below, <span className="warning">and keep your headset plugged in.</span> Open the game and do whatever it is that was causing you issues, then click the button again.</p>

        <p className="warning"></p>
        {!logging ? 
            <button onClick={async () => setLogging(true)}>Start Logging</button> : 
            <button onClick={() => setLogging(false)}>Stop Logging</button>}
            <br/>

        {waitingForLog && <p>Please wait while the log file generates . . .</p>}
        {logFile !== null && <a href={URL.createObjectURL(logFile)} download={"logcat.log"}><button>Download Log</button></a>}
    </>
}