import { useState } from 'react';
import '../css/CornerMenu.css';
import PreferencesIcon from '../icons/preferences.svg';
import SparklesIcon from '../icons/sparkles.svg';
import { CreditsModal } from './CreditsModal';
import { SettingsModal } from './SettingsModal';
import { LanguagePicker } from './LanguagePicker';

export function CornerMenu() {
    const [creditsOpen, setCreditsOpen] = useState(false);
    const [settingsOpen, setSettingsOpen] = useState(false);

    return <>
        <div className="cornerMenu container">
            <div className="cornerMenuRow" onClick={() => setSettingsOpen(true)}>
                <p>Settings</p>
                <img src={PreferencesIcon} alt="Preferences icon" />
            </div>
            <div className="cornerMenuRow" onClick={() => setCreditsOpen(true)}>
                <p>Credits</p>
                <img src={SparklesIcon} alt="Preferences icon" />
            </div>
        </div>
        <LanguagePicker />
        <CreditsModal isVisible={creditsOpen} onClose={() => setCreditsOpen(false)} />
        <SettingsModal isVisible={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </>
}