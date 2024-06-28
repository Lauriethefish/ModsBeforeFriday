import { useState } from "react";

import "../css/SelectableList.css"

export function SelectableList({ options, choiceSelected }: { options: string[], choiceSelected: (choice: string) => void }) {
    const [selected, setSelected] = useState(null as string | null);

    return <div className="codeBox selectableList">
        {options.map(option => 
            <p key={option} onClick={() => {
                setSelected(option);
                choiceSelected(option);
            }} 
                className={selected === option ? "selectedListItem listItem" : "listItem"}>
        {option}</p>)}
    </div>
}