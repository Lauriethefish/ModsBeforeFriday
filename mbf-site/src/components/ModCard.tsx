import '../css/ModCard.css';
import { Mod, trimGameVersion } from '../Models'
import { Slider } from './Slider';
import TrashCan from '../icons/trash.svg';
import { YesNoModal } from './Modal';
import { useState } from 'react';

interface ModCardProps {
    mod: Mod,
    gameVersion: string,
    onEnabledChanged: (enabled: boolean) => void,
    onRemoved: () => void
}

export function ModCard(props: ModCardProps) {
    const [requestRemove, setRequestRemove] = useState(false);
    const [wrongGameVersion, setWrongGameVersion] = useState(false);
    const setEnabled = (enabled: boolean) => {
        props.onEnabledChanged(enabled);
        props.mod.is_enabled = enabled;
    }

    return <div className="container modCard">
        <div className='modName'>
            <p className='nameText'>{props.mod.name}</p>
            <p className='idVersionText'>{props.mod.id} v{props.mod.version}</p>
        </div>

        <p className='descriptionText'>{props.mod.description}</p>

        <div className='modControls'>
            <div id="removeMod" onClick={() => setRequestRemove(true)}>
                <img src={TrashCan} alt="Remove mod icon" />
            </div>
            <Slider on={props.mod.is_enabled} valueChanged={value => {
                if(value && props.mod.game_version != null 
                    && props.mod.game_version !== props.gameVersion) {
                    setWrongGameVersion(true);
                }   else    {
                    console.log(props.mod.game_version, props.gameVersion);
                    setEnabled(value);
                }
            }}/>
        </div>

        <YesNoModal
            title="Confirm"
            onYes={() => {
                setRequestRemove(false);
                props.onRemoved();
            }}
            onNo={()=> setRequestRemove(false)}
            isVisible={requestRemove}>
            <p>Are you sure that you want to remove {props.mod.id} v{props.mod.version}?</p>
        </YesNoModal>
        <YesNoModal title="Wrong game version"
            onYes={() => { setEnabled(true); setWrongGameVersion(false) }}
            onNo={() => setWrongGameVersion(false)} 
            isVisible={wrongGameVersion}>
            <p>The mod {props.mod.id} v{props.mod.version} is designed for game version {props.mod.game_version === null ? null : trimGameVersion(props.mod.game_version)} but you have {trimGameVersion(props.gameVersion)}.</p>
            <p className="warning">It is EXTREMELY likely that enabling it will crash your game and/or mess up your mods in a way that could be VERY DIFFICULT to undo.</p>
            <p>Are you sure you still want to enable it (you don't)?</p>
        </YesNoModal>
    </div>
}