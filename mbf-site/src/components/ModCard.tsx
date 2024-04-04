import '../css/ModCard.css';
import { Mod } from '../Models'
import { Slider } from './Slider';
import TrashCan from '../icons/trash.svg';
import { ConfirmModal, Modal } from './Modal';
import { useState } from 'react';

interface ModCardProps {
    mod: Mod,
    onEnabledChanged: (enabled: boolean) => void,
    onRemoved: () => void
}

export function ModCard(props: ModCardProps) {
    const [requestRemove, setRequestRemove] = useState(false);

    return <div className="container modCard">
        <div className='modName'>
            <p className='nameText'>{props.mod.name}</p>
            <p className='idVersionText'>{props.mod.id} v{props.mod.version}</p>
        </div>

        <p className='descriptionText'>{props.mod.description}</p>

        <div className='modControls'>
            <div id="removeMod" onClick={() => setRequestRemove(true)}>
                <img src={TrashCan} />
            </div>
            <Slider on={props.mod.is_enabled} valueChanged={value => {
                props.onEnabledChanged(value);
                props.mod.is_enabled = value;
            }}/>
        </div>

        <ConfirmModal
            onConfirm={() => {
                setRequestRemove(false);
                props.onRemoved();
            }}
            onDeny={()=> setRequestRemove(false)}
            isVisible={requestRemove}>
            <p>Are you sure that you want to remove {props.mod.id} v{props.mod.version}?</p>
        </ConfirmModal>
    </div>
}