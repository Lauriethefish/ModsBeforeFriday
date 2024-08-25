

// Used to keep track of the current operation that MBF is carrying out.
// "Operations" are mutually exclusive, so e.g. if the app permissions are being changed,
// we cannot have a mod installation in progress, for example.
import { create } from "zustand";
import { Log } from "./Logging";

// An error that occured within a particular operation.
// 
export interface OperationError {
    title: string,
    error: string
}

export interface SyncStore {
    // Name of the current ongoing operation
    currentOperation: string | null;
    // Progress/status text for the ongoing operation
    statusText: string | null;
    currentError: OperationError | null,
    // Whether or not the logs have been manually opened by the user.
    logsManuallyOpen: boolean,
    // Whether to open a modal during sync.
    // Before the modding status is loaded, the progress of the operation
    // is shown in the main UI instead of in a modal. Therefore, this property is set to `false`
    // to avoid covering this with a modal.
    showSyncModal: boolean,

    setOperation: (operation: string | null) => void,
    setError: (error: OperationError | null) => void,
    setLogsManuallyOpen: (manuallyOpen: boolean) => void,
    setStatusText: (text: string | null) => void,
}

export const useSyncStore = create<SyncStore>(set => ({
    currentOperation: null,
    currentError: null,
    logsManuallyOpen: false,
    showSyncModal: false,
    statusText: null,
    setOperation: (operation: string | null) => set(_ => ({ currentOperation: operation })),
    setError: (error: OperationError | null) => set(_ => ({ currentError: error })),
    setLogsManuallyOpen: (manuallyOpen: boolean) => set(_ => ({ logsManuallyOpen: manuallyOpen })),
    setStatusText: (text: string | null) => set(_ => ({ statusText: text }))
}));

// Creates a function that can be used to set whether or not a particular operation is currently in progress.
export function useSetWorking(operationName: string): (working: boolean) => void {
    const { setOperation, setStatusText } = useSyncStore.getState();

    return working => {
        if(working) {
            setOperation(operationName);
        }   else    {
            setStatusText(null);
            setOperation(null);
        }
    }
}

// Creates a function that can be used to set an error when a particular operation failed.
export function useSetError(errorTitle: string): (error: unknown | null) => void {
    const { setError } = useSyncStore.getState();

    return error => {
        if(error === null) {
            setError(null);
        }   else    {
            Log.error(errorTitle + ": " + String(error));
            setError({
                title: errorTitle,
                error: String(error)
            })
        }
    }
}

// Used to wrap a particular operation while displaying the logging window and any errors if appropriate.
export async function wrapOperation(operationName: string,
    errorModalTitle: string,
    operation: () => Promise<void>) {
    const setWorking = useSetWorking(operationName);
    const setError = useSetError(errorModalTitle);

    setWorking(true);
    try {
        await operation();
    }   catch(error) {
        Log.error(errorModalTitle + ": " + error);
        setError(error);
    }   finally {
        setWorking(false);
    }
}