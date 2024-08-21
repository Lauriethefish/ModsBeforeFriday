

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
    currentOperation: string | null;
    currentError: OperationError | null,
    setOperation: (operation: string | null) => void,
    setError: (error: OperationError | null) => void,
}

export const useSyncStore = create<SyncStore>(set => ({
    currentOperation: null,
    currentError: null,
    setOperation: (operation: string | null) => set(_ => ({ currentOperation: operation })),
    setError: (error: OperationError | null) => set(_ => ({ currentError: error }))
}));

// Creates a function that can be used to set whether or not a particular operation is currently in progress.
export function useSetWorking(operationName: string): (working: boolean) => void {
    const { setOperation } = useSyncStore.getState();

    return working => {
        if(working) {
            setOperation(operationName);
        }   else    {
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
        setError(error);
    }   finally {
        setWorking(false);
    }
}