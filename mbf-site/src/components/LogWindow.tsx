import { useEffect } from 'react';
import '../css/LogWindow.css';
import '../fonts/Consolas.ttf';
import AlertTriangle from '../icons/alert-triangle.svg';
import AlertCircle from '../icons/alert-circle.svg';
import { Log, useLogStore } from '../Logging';
import { LogMsg } from '../Messages';
import DebugIcon from '../icons/debug.svg';
import CopyIcon from '../icons/copy.svg';
import QuitIcon from '../icons/exit.svg';
import { IconButton } from './IconButton';
import { toast } from 'react-toastify';

export function LogItem({ event }: { event: LogMsg }) {
    switch(event.level) {
        case 'Warn':
            return <span className="logItem logWithIcon logWarning">
                <img src={AlertTriangle} alt="A warning triangle" width="20" />
                {event.message}
            </span>
        case 'Error':
            return <span className="logItem logWithIcon logError">
                <img src={AlertCircle} alt="An error icon" width="20" />
                {event.message}
            </span>
        case 'Debug':
        case 'Trace':
            return <p className="logItem logDebug">{event.message}</p>
        default:
            return <p className="logItem">{event.message}</p>
    }
}

export function LogWindow() {
    const { logEvents, enableDebugLogs } = useLogStore();

    // Ensure that the logs always get scrolled to the bottom.
    let bottomDiv: Element | null = null;
    useEffect(() => {
        bottomDiv?.scrollIntoView();
    })

    return <div className="logWindowParent">
        <div className="codeBox logWindow">
            {logEvents
                // Filter out debug logs if these are disabled.
                .filter(event => {
                    const isDebug = event.level == 'Debug' || event.level == 'Trace';

                    return !isDebug || enableDebugLogs;
                })
                .map((event, idx) => <LogItem event={event} key={idx} />)}

            <div ref={element => { bottomDiv = element }}/>
        </div>
    </div>
}

export function LogWindowControls({ onClose }: { onClose?: () => void}) {
    const { enableDebugLogs, setEnableDebugLogs } = useLogStore();

    return <div className="logWindowControls">
        <IconButton src={CopyIcon} iconSize={25} alt="Copy Logs to clipboard" onClick={async () => copyLogsToClipboard()}/>
        <IconButton src={DebugIcon} iconSize={25} alt="Enable Debug Logs"
            onClick={() => setEnableDebugLogs(!enableDebugLogs)}
            isOn={enableDebugLogs}/>
        {onClose && <IconButton src={QuitIcon} iconSize={25} alt="Close log window" onClick={onClose} warning={true} />}
    </div>
}

async function copyLogsToClipboard() {
    await navigator.clipboard.writeText(Log.getLogsAsString());
    toast.success("Copied logs to clipboard");
}