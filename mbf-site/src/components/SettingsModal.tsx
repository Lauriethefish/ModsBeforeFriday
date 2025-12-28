import { useState } from "react";
import { Modal } from "./Modal";
import { Slider } from "./Slider";
import { getBgUserPreference, setBgUserPreference } from "../AnimatedBackground";
import { getLang } from "../localization/shared";

export function SettingsModal({ isVisible, onClose }: { isVisible: boolean, onClose: () => void }) {
    const [bgEnabled, setBgEnabled] = useState(getBgUserPreference());

    return <Modal isVisible={isVisible}>
        <h2>{getLang().settings}</h2>
        <div className="horizontalCenter">
            <p>{getLang().showAnimatedBackground}</p>        
            <Slider on={bgEnabled} valueChanged={enabled => {
                setBgEnabled(enabled);
                setBgUserPreference(enabled);
            }}/>
        </div>

        <div className="confirmButtons">
            <button onClick={onClose}>OK</button>
        </div>
    </Modal>
}