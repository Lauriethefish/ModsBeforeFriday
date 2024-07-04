
import { useState } from 'react';
import '../css/EditableList.css';

// A component that allows adding/removing items from an array.
// The items in the array must be distinct.
export function EditableList({ list, addItem, removeItem, title }: 
    { list: string[], addItem: (item: string) => void, removeItem: (item: string) => void, title: string}) {
    

    const [isAdding, setAdding] = useState(false);
    const [newValue, setNewValue] = useState("");

    return <>
        <span id="listTitle">
            <h4>{title}</h4>
            <button onClick={() => setAdding(true)}>+</button>
        </span>


        <div id="editableList" className="codeBox">
            {list.length > 0 || isAdding ? <>
                {list.map(item => 
                    <p key={item}
                        onClick={() => removeItem(item)}>
                        {item}
                    </p>
                )}
            </> : <>...</>}

            {isAdding && <input type="text" autoFocus
                pattern="[A-Za-z0-9._]*"
                onChange={ev => {
                    if(ev.target.validity.valid) {
                        setNewValue(ev.target.value);
                    }   else    {
                        setNewValue("");
                    }
                }}
                onBlur={_ => {
                    if(newValue !== "" && !list.includes(newValue)) {
                        addItem(newValue);
                    }
                    setNewValue("");
                    setAdding(false);
                }}/>}
        </div>
    </>
}