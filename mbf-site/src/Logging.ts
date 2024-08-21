import { LogMsg } from "./Messages";
import { create } from 'zustand';

export interface LogEventStore {
    logEvents: LogMsg[];
    addLogEvent: (msg: LogMsg) => void
}

// Used to globally distribute the MBF log messages.
export const useLogStore = create<LogEventStore>((set) => ({
    logEvents: [],
    addLogEvent: (msg: LogMsg) => set((state) => ({ logEvents: [...state.logEvents, msg]} ))
}))