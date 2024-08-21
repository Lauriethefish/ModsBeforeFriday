import { useContext, useEffect } from 'react';
import '../css/LogWindow.css';
import { LogMsg } from '../Messages';
import '../fonts/Consolas.ttf';
import { useLogStore } from '../Logging';


export function LogWindow() {
    const { logEvents } = useLogStore();

    // Ensure that the logs always get scrolled to the bottom.
    let bottomDiv: Element | null = null;
    useEffect(() => {
        bottomDiv?.scrollIntoView();
    })

    return <div id="logWindow" className="codeBox">
        {logEvents.map((event, idx) => <p className="logItem" key={idx}>{event.message}</p>)}

        <div ref={element => { bottomDiv = element }}/>
    </div>
}