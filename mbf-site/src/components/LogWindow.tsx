import { useEffect, useState } from 'react';
import '../css/LogWindow.css';
import { LogMsg } from '../Messages';
import '../fonts/Consolas.ttf';

interface LogWindowProps {
    events: LogMsg[]
}

// Convenience function to set up the state necessary for a logging window.
export function useLog(): [events: LogMsg[], addEvent: (event: LogMsg) => void] {
    // This is a little cursed but gets the job done for now.
    let [logEvents, setLogEvents] = useState([] as LogMsg[]);

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
    let bottomDiv: Element | null = null;
    useEffect(() => {
        bottomDiv?.scrollIntoView();
    })

    return <div id="logWindow">
        {props.events.map((event, idx) => <p className="logItem" key={idx}>{event.message}</p>)}

        <div ref={element => { bottomDiv = element }}/>
    </div>
}