import { LogMsg } from "./Messages";
import { create } from 'zustand';

export interface LogEventStore {
    logEvents: LogMsg[];
    addLogEvent: (msg: LogMsg) => void,
    enableDebugLogs: boolean,
    setEnableDebugLogs: (enabled: boolean) => void
}

// Used to globally distribute the MBF log messages.
export const useLogStore = create<LogEventStore>(set => ({
    logEvents: [],
    enableDebugLogs: import.meta.env.DEV,
    addLogEvent: (msg: LogMsg) => set((state) => ({ logEvents: [...state.logEvents, msg]} )),
    setEnableDebugLogs: (enabled: boolean) => set(_ => ({ enableDebugLogs: enabled }))
}))

interface MbfLogMsg<T> extends LogMsg {
    rawData?: T
}

// Logging class which provides convenience functions to manipulate the global logging state.
export class Log {
    static emitEvent<T>(event: MbfLogMsg<T>) {
        // Log the event to the console, more convenient during MBF development.
        let consoleData: any[] = [event.message];

        if (event.rawData !== undefined) {
            consoleData.push(event.rawData);
        }

        switch(event.level) {
            case 'Error':
                console.error(...consoleData);
                break;
            case 'Warn':
                console.warn(...consoleData);
                break;
            case 'Debug':
                console.debug(...consoleData);
                break;
            case 'Info':
                console.info(...consoleData);
                break;
            case 'Trace':
                console.trace(...consoleData);
        }

        // Log the event to the global log store.
        delete event.rawData;
        useLogStore.getState().addLogEvent(event);
    }

    // Gets a large string containing all messages logged to MBF.
    static getLogsAsString(): string {
        let logs = "";
        useLogStore.getState().logEvents.forEach(event => {
            logs += `${event.level.toUpperCase()}> ${event.message}\n`;
        });

        return logs;
    }

    static emitMessage<T>(level: 'Trace' | 'Debug' | 'Info' | 'Warn' | 'Error', msg: any, raw?: T) {
        let rawData: any;

        if (raw !== undefined) {
            rawData = raw;
        }

        this.emitEvent({
            'type': 'LogMsg',
            'level': level,
            message: String(msg),
            rawData: rawData
        });
    }

    static trace: (<T>(msg: any, raw?: T) => void) = (msg, raw) => this.emitMessage('Trace', msg, raw);
    static debug: (<T>(msg: any, raw?: T) => void) = (msg, raw) => this.emitMessage('Debug', msg, raw);
    static info: (<T>(msg: any, raw?: T) => void) = (msg, raw) => this.emitMessage('Info', msg, raw);
    static warn: (<T>(msg: any, raw?: T) => void) = (msg, raw) => this.emitMessage('Warn', msg, raw);
    static error: (<T>(msg: any, raw?: T) => void) = (msg, raw) => this.emitMessage('Error', msg, raw);
}