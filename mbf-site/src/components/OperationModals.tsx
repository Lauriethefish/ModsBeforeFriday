import { useSyncStore } from "../SyncStore";
import { ErrorModal, SyncingModal } from "./Modal";

// Component that displays the log window when an operation is in progress, and displays errors when the operation failed.
export function OperationModals() {
    const { currentOperation, currentError, setError } = useSyncStore();

    return <>
        <SyncingModal isVisible={currentOperation !== null} title={currentOperation ?? ""} />
        <ErrorModal isVisible={currentError !== null} title={currentError?.title ?? ""} onClose={() => setError(null)}>
            {currentError?.error ?? ""}
        </ErrorModal>
    </>
}