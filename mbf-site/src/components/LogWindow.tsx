import { useState } from 'react';
import 'css/LogWindow.css';
import { Log } from '../Messages';
import './fonts/Consolas.ttf';

interface LogWindowProps {
    events: Log[]
}

// Convenience function to set up the state necessary for a logging window.
export function useLog(): [events: Log[], addEvent: (event: Log) => void] {
    // This is a little cursed but gets the job done for now.
    let [logEvents, setLogEvents] = useState([] as Log[]);

    return [
        logEvents,
        (event) => {
            // We MUST assign this back to the original variable
            // otherwise subsequent calls will assume that no events have yet been written
            // since THEIR logEvents will be the same as in the first call (empty)
            logEvents = [
                ...logEvents,
                event
            ];
            // Notify react that a new event has been added
            setLogEvents(logEvents);
        }
    ]
}

export function LogWindow(props: LogWindowProps) {
    return <div id="logWindow">
        {props.events.map((event, idx) => <p className="logItem" key={idx}>{event.message}</p>)}
    </div>
}