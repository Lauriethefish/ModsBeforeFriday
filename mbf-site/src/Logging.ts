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

// Logging class which provides convenience functions to manipulate the global logging state.
export class Log {
    static emitEvent(event: LogMsg) {
        useLogStore.getState().addLogEvent(event);

        // Also log the event to the console, more convenient during MBF development.
        switch(event.level) {
            case 'Error':
                console.error(event.message);
                break;
            case 'Warn':
                console.warn(event.message);
                break;
            case 'Debug':
                console.debug(event.message);
                break;
            case 'Info':
                console.info(event.message);
                break;
            case 'Trace':
                console.trace(event.message);
        }
    }

    static trace(msg: any) {
        this.emitEvent({
            'type': 'LogMsg',
            'level': 'Trace',
            message: String(msg)
        })
    }

    static debug(msg: any) {
        this.emitEvent({
            'type': 'LogMsg',
            'level': 'Debug',
            message: String(msg)
        })
    }

    static info(msg: any) {
        this.emitEvent({
            'type': 'LogMsg',
            'level': 'Info',
            message: String(msg)
        })
    }

    static warn(msg: any) {
        this.emitEvent({
            'type': 'LogMsg',
            'level': 'Warn',
            message: String(msg)
        })
    }

    static error(msg: any) {
        this.emitEvent({
            'type': 'LogMsg',
            'level': 'Error',
            message: String(msg)
        })
    }
}