import { toast } from 'react-toastify';
import { copyToClipboard } from '../copyToClipboard';
import CopyIcon from '../icons/copy.svg';
import '../css/CopyableCommand.css';

export function CopyableCommand({ command }: { command: string }) {
    async function copyCommand() {
        if (await copyToClipboard(command)) {
            toast.success('Command copied to clipboard');
        } else {
            toast.error('Could not copy the command. Please select and copy it manually.');
        }
    }

    return <div className="codeBox copyableCommand">
        <code>{command}</code>
        <button type="button" aria-label="Copy command to clipboard" title="Copy command" onClick={copyCommand}>
            <img src={CopyIcon} alt="" width={20} height={20} />
        </button>
    </div>;
}
