import '../css/IconButton.css';

// A button that displays an icon at its centre
export function IconButton( { src, alt, iconSize, onClick, isOn }: 
    { 
        src: string,
        alt: string,
        iconSize?: number
        onClick: () => void,
        isOn?: boolean,
    }) {
    return <div className={isOn ? "iconButton iconButtonOn" : "iconButton iconButtonOff"} onClick={onClick}>
        <img src={src} alt={alt} width={iconSize} />
    </div>
}