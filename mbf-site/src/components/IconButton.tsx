import '../css/IconButton.css';

// A button that displays an icon at its centre
export function IconButton( { src, alt, iconSize, onClick, isOn, warning }: 
    { 
        src: string,
        alt: string,
        iconSize?: number
        onClick: () => void,
        isOn?: boolean,
        // When enabled, the background of the button is red.
        warning?: boolean,
    }) {
    let className = "iconButton";
    if(warning) {
        className += " iconButtonWarning";
    }   else if(isOn) {
        className += " iconButtonOn";
    }   else    {
        className += " iconButtonOff";
    }

    return <div className={className} onClick={onClick}>
        <img src={src} alt={alt} width={iconSize} />
    </div>
}