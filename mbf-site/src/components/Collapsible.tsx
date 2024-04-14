import { ReactNode, useEffect, useRef, useState } from "react";

import '../css/Collapsible.css';

// A menu that can be collapsed and expanded with a mouse click.
export function Collapsible({ title, children, defaultOpen }: { title: string, children: ReactNode, defaultOpen?: boolean }) {
    const [expanded, setExpanded] = useState(defaultOpen);

    const content = useRef<HTMLDivElement | null>(null);
    const childrenDiv = useRef<HTMLDivElement | null>(null);

    const updateMaxHeight = () => {
        const currContent = content.current;
        if(currContent === null) {
            return;
        }

        if(expanded) {
            currContent.style.maxHeight = currContent.scrollHeight + "px";
        }   else    {
            currContent.style.maxHeight = "0";
        }
    };

    useEffect(() => {
        updateMaxHeight();

        // Make sure that if the child changes size, we detect this
        const observer = new ResizeObserver(updateMaxHeight);
        observer.observe(childrenDiv.current!);

        return () => observer.disconnect();
    });

    return <div className="collapsible">
        <div className="collapsibleTitle" onClick={() => setExpanded(!expanded) }>
            <h3>{title}</h3>
            <p>{expanded ? "-" : "+"}</p>
        </div>
        <div className="content" ref={content}>
            <div ref={childrenDiv}>
                {children}
            </div>
            <div id="bottom" />
        </div>
    </div>
}