
import { useState } from 'react';
import '../css/EditableList.css';

// A component that allows adding/removing items from an array.
// The items in the array must be distinct.
export function EditableList({ list, setList, title }: 
    { list: string[], setList: (newList: string[]) => void, title: string}) {
    

    const [isAdding, setAdding] = useState(false);
    const [newValue, setNewValue] = useState("");

    return <>
        <span id="listTitle">
            <h4>{title}</h4>
            <button onClick={() => setAdding(true)}>+</button>
        </span>


        <div id="editableList">
            {list.length > 0 || isAdding ? <>
                {list.map(item => 
                    <p key={item}
                        onClick={() => setList(list.filter(member => member != item))}>
                        {item}
                    </p>
                )}
            </> : <>(None)</>}

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
                        setList([...list, newValue]);
                    }
                    setNewValue("");
                    setAdding(false);
                }}/>}
        </div>
    </>
}