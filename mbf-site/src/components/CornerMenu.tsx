import { SourceUrl } from '..';
import '../css/CornerMenu.css';

export function CornerSourceLink() {
    return <div className="cornerMenu container">
        <p><b>MBF</b> is an app by <b>Lauriethefish</b></p>
        <a href={SourceUrl} target="_blank" rel="noopener noreferrer">View Source Code</a>
    </div>
}