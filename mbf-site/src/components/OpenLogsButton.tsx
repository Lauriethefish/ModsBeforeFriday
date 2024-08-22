import '../css/OpenLogs.css';
import LogsIcon from '../icons/logs.svg';
import { useSyncStore } from '../SyncStore';

export function OpenLogsButton() {
    const { setLogsManuallyOpen } = useSyncStore();

    return <button className="openLogs" onClick={() => setLogsManuallyOpen(true)}>
        View Logs
        <img src={LogsIcon} alt="Paper with lines of text on it" />
    </button>
}