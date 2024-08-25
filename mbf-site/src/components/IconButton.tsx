import '../css/IconButton.css';

// A button that displays an icon at its centre
// This button is designed for use as a part of a row of several icon buttons, e.g. in the logs menu.
// It is not designed to sit on its own and does not have a background unless hovered.
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