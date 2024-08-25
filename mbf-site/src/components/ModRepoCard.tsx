import { ModRepoMod } from "../ModsRepo";
import '../css/ModRepoCard.css';
import DownloadIcon from '../icons/download-icon.svg';
import ModIcon from '../icons/mod-icon.svg';
import { LabelledIconButton } from "./LabelledIconButton";
import CodeIcon from '../icons/code.svg';
import BugIcon from '../icons/debug.svg';
import FlagIcon from '../icons/flag.svg';
import FilledFlagIcon from '../icons/flag-filled.svg';
import { IconButton } from "./IconButton";

export function ModRepoCard({ mod, onInstall, update, isFlagged, setFlagged }: 
    { 
        mod: ModRepoMod,
        onInstall: () => void,
        update: boolean,
        isFlagged: boolean,
        setFlagged: (flagged: boolean) => void }) {

    // In the DB, the mod cover is either null, undefined or any empty string.
    // How fun that we get to check for all three!
    const hasCover = mod.cover !== null && mod.cover !== undefined && mod.cover.length > 0;
    return <div className="modRepoCard container">
        {hasCover && <img src={mod.cover!} className="cover" />}
        {!hasCover && <div className="defaultCover">
            <img src={ModIcon} width={40} />
        </div>}
        <div className="mod-repo-card-info">
            <span className="modDetails">
                <span className="bookmarkMod">
                    <IconButton src={isFlagged ? FilledFlagIcon : FlagIcon} alt="A flag"
                        onClick={() => setFlagged(!isFlagged)}
                        isOn={isFlagged}/>
                </span>
                {mod.name} v{mod.version}
                <p className="author">by {mod.author}</p>
            </span>
            <p>{mod.description}</p>
            <div className="auxOptions">
                <a href={mod.source} target="_blank">
                    <LabelledIconButton iconSrc={CodeIcon} iconAlt="Programming code" label="Source Code" noIconOnMobile/>
                </a>
                {mod.source.includes("github") && 
                    <a href={mod.source + "/issues"} target="_blank">
                        <LabelledIconButton iconSrc={BugIcon} iconAlt="A bug" label="Report bug" noIconOnMobile/>
                    </a>}

                <button className="installMod" onClick={onInstall}>
                    {update ? "Update" : "Install"}
                    <img src={DownloadIcon} />
                </button>
            </div>
        </div>
    </div>
}