import { SourceUrl } from "..";
import { Modal } from "./Modal";

export function CreditsModal({ isVisible, onClose }: { isVisible: boolean, onClose: () => void }) {
    return <Modal isVisible={isVisible}>
        <h2>Credits</h2>
        <p>Hi, it's <b>Lauriethefish</b> here, the original author of ModsBeforeFriday.</p>
        <p>MBF is an <a href={SourceUrl}>open source project</a>, and over the course of development, numerous people have stepped up to improve the app.</p>
        <p>It is important to remember that MBF is just <em>installing</em> your mods. There are many very talented people behind the core mods that MBF installs,
        and unless you've been paying close attention to the mod list, you won't even know many of their names!</p>
        <p>This menu solely focuses on people who have contributed to the MBF app.</p>

        <h3>MBF contributors</h3>
        <ul>
            <li><a href="https://github.com/DanTheMan827">DanTheMan827</a> implemented native adb connection through a WebSocket proxy server.</li>
            <li><a href="https://github.com/FrozenAlex">FrozenAlex</a> created the drag 'n' drop system for MBF, and has provided me with much insight on UI design. Without him, the UI would be (even more of) a cluttered mess!</li>
            <li><a href="https://github.com/XoToM">XoToM</a>, a good friend of mine, created the animated background that you know and love. (although your CPU might hate it!)</li>
            <li><a href="https://github.com/AltyFox">Alteran</a>, a member of the BSMG support team, has provided invaluable feedback regarding usability, and has helped me to pinpoint and fix bugs.</li>

        </ul>

        <div className="confirmButtons">
            <button onClick={onClose}>OK</button>
        </div>
    </Modal>
}