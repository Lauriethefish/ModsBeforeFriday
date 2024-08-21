import { useEffect } from 'react';
import '../css/LogWindow.css';
import '../fonts/Consolas.ttf';
import AlertTriangle from '../icons/alert-triangle.svg';
import AlertCircle from '../icons/alert-circle.svg';
import { useLogStore } from '../Logging';
import { LogMsg } from '../Messages';

export function LogItem({ event }: { event: LogMsg }) {
    switch(event.level) {
        case 'Warn':
            return <span className="logItem logWithIcon logWarning">
                <img src={AlertTriangle} alt="A warning triangle" width="20" />
                {event.message}
            </span>
        case 'Error':
            return <span className="logItem logWithIcon logError">
                <img src={AlertCircle} alt="An error icon" width="20" />
                {event.message}
            </span>
        case 'Debug':
        case 'Trace':
            return <p className="logItem logDebug">{event.message}</p>
        default:
            return <p className="logItem">{event.message}</p>
    }
}

export function LogWindow() {
    const { logEvents } = useLogStore();

    // Ensure that the logs always get scrolled to the bottom.
    let bottomDiv: Element | null = null;
    useEffect(() => {
        bottomDiv?.scrollIntoView();
    })

    return <div id="logWindow" className="codeBox">
        {logEvents.map((event, idx) => <LogItem event={event} key={idx} />)}

        <div ref={element => { bottomDiv = element }}/>
    </div>
}