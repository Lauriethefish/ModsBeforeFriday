import '../css/ModCard.css';
import { Mod, trimGameVersion } from '../Models'
import { Slider } from './Slider';
import TrashCan from '../icons/trash.svg';
import Code from '../icons/code.svg';
import { YesNoModal } from './Modal';
import { useState } from 'react';

interface ModCardProps {
    mod: Mod,
    gameVersion: string,
    onEnabledChanged: (enabled: boolean) => void,
    onRemoved: () => void
    pendingChange?: boolean
}

function CoreModBadge() {
    return <div className="coreBadge">
        <img src={Code} alt="Code symbol"/>
        <p>CORE</p>
    </div>
}

function CoreModWarning(props: { mod: Mod }) {
    return <>
        <p className="warning">{props.mod.name} is a <em>core mod</em>.</p>
        <p>
            These mods are automatically installed and managed by MBF. Removing or disabling them will most likely <b>prevent
            your custom songs from loading</b> or have other <b>unforseen consequences.</b>
        </p>
        <p className="warning">
            You have been warned.
        </p>
    </>;
}

export function ModCard(props: ModCardProps) {
    const [requestRemove, setRequestRemove] = useState(false);
    const [requestDisable, setRequestDisable] = useState(false);
    const [wrongGameVersion, setWrongGameVersion] = useState(false);
    const setEnabled = (enabled: boolean) => {
        props.onEnabledChanged(enabled);
        props.mod.is_enabled = enabled;
    }

    return <div className="container modCard">
        <div className='modName'>
            <span className="nameSpan">
                <p className='nameText'>{props.mod.name}</p>
                {props.mod.is_core && <CoreModBadge />}
            </span>
            <p className='idVersionText'>{props.mod.id} v{props.mod.version}</p>
        </div>

        <p className='descriptionText'>{props.mod.description}</p>

        <div className='modControls'>
            <div id="removeMod" onClick={() => setRequestRemove(true)}>
                <img src={TrashCan} alt="Remove mod icon" />
            </div>
            <Slider on={props.pendingChange !== undefined ? props.pendingChange : props.mod.is_enabled} valueChanged={value => {
                if(value && props.mod.game_version != null 
                    && props.mod.game_version !== props.gameVersion
                    && !props.mod.is_core) { // Do not show the wrong game version prompt for core mods.
                        // This is because modders sometimes forget to update the game version, but if the mod is core
                        // then we know it's designed for the current version anyway, so there's no need for the prompt.
                    setWrongGameVersion(true);
                }   else    {
                    if(!value && props.mod.is_core) {
                        setRequestDisable(true);
                    }   else    {
                        setEnabled(value);
                    }
                }
            }}/>
        </div>

        <YesNoModal
            title={props.mod.is_core ? "Remove core mod " : "Confirm removal"}
            onYes={() => {
                setRequestRemove(false);
                props.onRemoved();
            }}
            onNo={()=> setRequestRemove(false)}
            isVisible={requestRemove}>
            {props.mod.is_core && <CoreModWarning mod={props.mod} />}

            <p>Are you sure that you want to remove {props.mod.name} v{props.mod.version}?</p>
        </YesNoModal>
        <YesNoModal title="Disable core mod"
            onYes={() => {
                setRequestDisable(false);
                setEnabled(false);
            }}
            onNo={() => setRequestDisable(false)}
            isVisible={requestDisable}>
            <CoreModWarning mod={props.mod} />

            <p>Are you still sure that you want to disable {props.mod.name} v{props.mod.version}?</p>
        </YesNoModal>

        <YesNoModal title="Wrong game version"
            onYes={() => { setEnabled(true); setWrongGameVersion(false) }}
            onNo={() => setWrongGameVersion(false)} 
            isVisible={wrongGameVersion}>
            <p>The mod {props.mod.name} v{props.mod.version} is designed for game version {props.mod.game_version === null ? null : trimGameVersion(props.mod.game_version)} but you have {trimGameVersion(props.gameVersion)}.</p>
            <p className="warning">It is EXTREMELY likely that enabling it will crash your game and mess up your mods in a way that could be VERY DIFFICULT to undo.</p>
            <p>Are you sure you still want to enable it (you don't)?</p>
        </YesNoModal>
    </div>
}