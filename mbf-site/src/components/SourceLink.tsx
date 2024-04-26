import '../css/SourceLink.css';

export const SourceUrl: string = "https://github.com/Lauriethefish/ModsBeforeFriday";

export function CornerSourceLink() {
    return <div className="sourceLink container">
        <p><b>MBF</b> is an app by <b>Lauriethefish</b></p>
        <a href={SourceUrl} target="_blank" rel="noopener noreferrer">View Source Code</a>
    </div>
}

export function SmallSourceLink() {
    return <a href={SourceUrl} target="_blank" rel="noopener noreferrer" className="mobileOnly">View Source Code</a>;
}