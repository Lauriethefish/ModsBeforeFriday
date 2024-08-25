import '../css/LabelledIconButton.css';

// A button that displays an icon and an associated label.
// It has the same background tint as a .discreetButton
export function LabelledIconButton({ label, iconSrc, iconAlt, onClick }: {
    label: string,
    iconSrc: string,
    iconAlt: string,
    onClick: () => void
}) {
    return <button className="discreetButton labelledIconButton" onClick={onClick}>
        {label}
        <img src={iconSrc} alt={iconAlt} />
    </button>
}