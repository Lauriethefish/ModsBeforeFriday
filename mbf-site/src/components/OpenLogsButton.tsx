import '../css/OpenLogs.css';
import LogsIcon from '../icons/logs.svg';
import { LabelledIconButton } from './LabelledIconButton';
import { useOperationModalsContext } from './OperationModals';

export function OpenLogsButton() {
    const { setLogsManuallyOpen } = useOperationModalsContext();

    return <div className="openLogs">
        <LabelledIconButton iconSrc={LogsIcon} iconAlt="Piece of paper with lines of text"
            label="Logs"
            onClick={() => setLogsManuallyOpen(true)}/>
    </div>
}