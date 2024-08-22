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

    return <>
        <SyncingModal isVisible={(currentOperation !== null || logsManuallyOpen) && currentError === null}
            title={currentOperation ?? "Log output"}
            onClose={canClose ? () => setLogsManuallyOpen(false) : undefined} />
        <ErrorModal isVisible={currentError !== null} title={currentError?.title ?? ""} onClose={() => setError(null)}>
            {currentError?.error ?? ""}
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
                    </div>

                    {/* Disable the controls built into the log window and instead put the controls to the right of the heading*/}
                    <LogWindowControls onClose={onClose} />
                    <LogWindow showControls={false} />
                </div>
            </div>
        </div>
    }   else   {
        return <div className="modalBackground modalClosed coverScreen"></div>
    }
}