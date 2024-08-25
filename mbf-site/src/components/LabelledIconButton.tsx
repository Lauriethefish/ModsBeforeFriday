import '../css/LabelledIconButton.css';
import { isViewingOnMobile } from '../platformDetection';

// A button that displays an icon and an associated label.
// It has the same background tint as a .discreetButton
export function LabelledIconButton({ label, iconSrc, iconAlt, onClick, noIconOnMobile }: {
    label: string,
    iconSrc: string,
    iconAlt: string,
    noIconOnMobile?: boolean,
    onClick?: () => void
}) {
    return <button className="discreetButton labelledIconButton" onClick={onClick}>
        {label}
        {!(isViewingOnMobile() && noIconOnMobile) && <img src={iconSrc} alt={iconAlt} width={22} />}
    </button>
}