

// Used to keep track of the current operation that MBF is carrying out.
// "Operations" are mutually exclusive, so e.g. if the app permissions are being changed,
// we cannot have a mod installation in progress, for example.
import { create } from "zustand";


export interface SyncStore {
    currentOperation: string | null;
    setOperation: (operation: string | null) => void,
}

export const useSyncStore = create<SyncStore>(set => ({
    currentOperation: null,
    setOperation: (operation: string | null) => set(_ => ({ currentOperation: operation })) 
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