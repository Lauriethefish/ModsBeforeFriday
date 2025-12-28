import '../css/OpenLogs.css';
import LogsIcon from '../icons/logs.svg';
import { LabelledIconButton } from './LabelledIconButton';
import { OperationModals } from './OperationModals';

export function OpenLogsButton() {

    return <div className="openLogs">
        <LabelledIconButton iconSrc={LogsIcon} iconAlt="Piece of paper with lines of text"
            label="Logs"
            onClick={() => OperationModals.logsManuallyOpen = true}/>
    </div>
}