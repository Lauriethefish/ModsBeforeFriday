import { SourceUrl } from "..";
import { getLang } from "../localization/shared";
import { Modal } from "./Modal";

export function CreditsModal({ isVisible, onClose }: { isVisible: boolean, onClose: () => void }) {
    return <Modal isVisible={isVisible}>
        <h2>{getLang().credits}</h2>
        {getLang().creditsIntro(SourceUrl)}

        <h3>{getLang().mbfContributors}</h3>
        <ul>
            <li><a href="https://github.com/FrozenAlex">FrozenAlex</a>{getLang().contributorIntroFrozenAlex}</li>
            <li><a href="https://github.com/XoToM">XoToM</a>{getLang().contributorXoToM}</li>
            <li><a href="https://github.com/AltyFox">Alteran</a>{getLang().contributorAltyFox}</li>
            {getLang().contributorLocalization}
        </ul>

        <div className="confirmButtons">
            <button onClick={onClose}>{getLang().creditsOkBtnText}</button>
        </div>
    </Modal>
}