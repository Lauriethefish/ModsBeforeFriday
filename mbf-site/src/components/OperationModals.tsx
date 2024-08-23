import { ScaleLoader } from "react-spinners";
import { useSyncStore } from "../SyncStore";
import { LogWindow, LogWindowControls } from "./LogWindow";
import { ErrorModal } from "./Modal";

// Component that displays the log window when an operation is in progress, and displays errors when the operation failed.
export function OperationModals() {
    const { currentOperation,
        currentError,
        setError,
        logsManuallyOpen,
        setLogsManuallyOpen } = useSyncStore();

    const canClose = logsManuallyOpen && currentError === null;
    const needSyncModal = (logsManuallyOpen || currentOperation !== null) 
        && currentError === null;

    return <>
        <SyncingModal isVisible={needSyncModal}
            title={currentOperation ?? "Log output"}
            onClose={canClose ? () => setLogsManuallyOpen(false) : undefined} />
        <ErrorModal isVisible={currentError !== null}
            title={currentError?.title ?? ""}
            description={currentError?.error}
            onClose={() => setError(null)}>
        </ErrorModal>
    </>
}


function SyncingModal({ isVisible, title, onClose }: { isVisible: boolean, title: string, onClose?: () => void }) {
    if(isVisible) {
        return  <div className="modalBackground coverScreen">
            <div className="modal container screenWidth">
                <div className="syncingWindow">

                    <div className="syncingTitle">
                        <h2>{title}</h2>
                        {onClose === undefined && <ScaleLoader color={"white"} height={20} />}
                        <LogWindowControls onClose={onClose} />
                    </div>
                    <LogWindow />
                </div>
            </div>
        </div>
    }   else   {
        return <div className="modalBackground modalClosed coverScreen"></div>
    }
}