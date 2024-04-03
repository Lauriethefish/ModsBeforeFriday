import { useState } from 'react';
import '../css/ModCard.css';
import { Mod } from '../Models'
import { Slider } from './Slider';

interface ModCardProps {
    mod: Mod,
    onEnabledChanged: (enabled: boolean) => void
}

export function ModCard(props: ModCardProps) {
    return <div className="container modCard">
        <div className='modName'>
            <p className='nameText'>{props.mod.name}</p>
            <p className='idVersionText'>{props.mod.id} v{props.mod.version}</p>
        </div>

        <p className='descriptionText'>{props.mod.description}</p>

        <div className='modToggle'>
            <Slider on={props.mod.is_enabled} valueChanged={value => {
                props.onEnabledChanged(value);
                props.mod.is_enabled = value;
            }}/>
        </div>
    </div>
}