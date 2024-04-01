import { useState } from 'react';
import './ModCard.css';
import { Mod } from './Models'
import { Slider } from './Slider';

interface ModCardProps {
    mod: Mod
}

export function ModCard(props: ModCardProps) {
    const [enabled, setEnabled] = useState(false);

    return <div className="container modCard">
        <div className='modName'>
            <p className='nameText'>{props.mod.name}</p>
            <p className='idVersionText'>{props.mod.id} v{props.mod.version}</p>
        </div>

        <p className='descriptionText'>{props.mod.description}</p>

        <div className='modToggle'>
            <Slider on={enabled} valueChanged={value => setEnabled(value)}/>
        </div>
    </div>
}