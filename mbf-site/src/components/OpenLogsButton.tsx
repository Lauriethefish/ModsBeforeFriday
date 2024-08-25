import '../css/OpenLogs.css';
import LogsIcon from '../icons/logs.svg';
import { useSyncStore } from '../SyncStore';
import { LabelledIconButton } from './LabelledIconButton';

export function OpenLogsButton() {
    const { setLogsManuallyOpen } = useSyncStore();

    return <div className="openLogs">
        <LabelledIconButton iconSrc={LogsIcon} iconAlt="Piece of paper with lines of text"
            label="Logs"
            onClick={() => setLogsManuallyOpen(true)}/>
    </div>
}