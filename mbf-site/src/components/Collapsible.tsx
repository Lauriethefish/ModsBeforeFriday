import { ReactNode, useState } from "react";

import '../css/Collapsible.css';

// A menu that can be collapsed and expanded with a mouse click.
export function Collapsible({ title, children }: { title: string, children: ReactNode }) {
    const [expanded, setExpanded] = useState(true);

    return <div className="collapsible">
        <div className="collapsibleTitle" onClick={() => setExpanded(!expanded)}>
            <h3>{title}</h3>
            <p>{expanded ? "-" : "+"}</p>
        </div>
        <div className={`content ${expanded ? "open" : "collapsed"}`}>
            {children}
            <div id="bottom" />
        </div>
    </div>
}