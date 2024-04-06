import { ModRepoMod } from "../ModsRepo";
import '../css/ModRepoCard.css';
import DownloadIcon from '../icons/download-icon.svg';
import ModIcon from '../icons/mod-icon.svg';

export function ModRepoCard({ mod, onInstall, update }: { mod: ModRepoMod, onInstall: () => void, update: boolean }) {
    // In the DB, the mod cover is either null, undefined or any empty string.
    // How fun that we get to check for all three!
    const hasCover = mod.cover !== null && mod.cover !== undefined && mod.cover.length > 0;
    return <div className="container" id="modRepoCard">
        {hasCover && <img src={mod.cover!} id="cover" />}
        {!hasCover && <div id="defaultCover">
            <img src={ModIcon} width={40} />
        </div>}
        <div>
            <p id="modDetails">{mod.name} v{mod.version}</p>
            <p id="author">by {mod.author}</p>
            <p>{mod.description}</p>
            <div id="auxOptions">
                <a href={mod.source} target="_blank"><button>View source code</button></a>
                {mod.source.includes("github") && 
                    <a href={mod.source + "/issues"} target="_blank"><button>Report a bug</button></a>}
            </div>
        </div>
        <button id="installMod" onClick={onInstall}>
            {update ? "Update" : "Install"}
            <img src={DownloadIcon} />
        </button>
    </div>
}