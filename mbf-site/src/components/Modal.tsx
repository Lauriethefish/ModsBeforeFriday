import { ReactNode, useEffect } from "react";
import { create } from "zustand";
import AlertIcon from '../icons/alert-triangle.svg'

import '../css/Modal.css';

interface ModalProps {
    children: ReactNode,
    isVisible: boolean
}

// Simple modal view with a card in the middle of the screen.
// Fades in so if the modal appears only briefly, there's no "UI flashing"
export function Modal(props: ModalProps) {
    if(props.isVisible) {
        return  <div className="modalBackground">
        <div className="modal container">
            {props.children}
        </div>
    </div>
    }   else   {
        return <div className="modalBackground modalClosed"></div>
    }
}

interface ErrorModalProps {
    isVisible: boolean,
    title: string,
    description?: string | null,
    children?: ReactNode,
    onClose: () => void
}

export function ErrorModal(props: ErrorModalProps) {
    return <Modal isVisible={props.isVisible}>
        <div id="errorTitle">
            <img src={AlertIcon} alt="A warning triangle" />
            <h1>{props.title}</h1>
        </div>
        <div>
            {props.description?.split('\n').map(line => <p key={line}>{line}</p>)}
            {props.children}
        </div>

        <div className="confirmButtons">
            <button onClick={props.onClose}>OK</button>
        </div>
    </Modal>
}

interface YesNoModalProps {
    isVisible: boolean,
    title: string,
    onYes: () => void,
    onNo: () => void,
    children: ReactNode
}

export function YesNoModal(props: YesNoModalProps) {
    return <Modal isVisible={props.isVisible}>
        <h1>{props.title}</h1>
        {props.children}
        <div className="confirmButtons">
            <button type="button" onClick={props.onYes}>Yes</button>
            <button type="button" onClick={props.onNo}>No</button>
        </div>
    </Modal>
}

interface Confirmation {
    title: string,
    children: ReactNode,
    resolve: (confirmed: boolean) => void
}

const useConfirmationStore = create<{ queue: Confirmation[] }>(() => ({ queue: [] }));

// Requests are queued so overlapping callers each receive their own answer.
export function confirm(title: string, children: ReactNode): Promise<boolean> {
    return new Promise(resolve => {
        useConfirmationStore.setState(state => ({
            queue: [...state.queue, { title, children, resolve }]
        }));
    });
}

// Dismiss requests when their application context is no longer valid.
export function cancelConfirmations() {
    const { queue } = useConfirmationStore.getState();
    useConfirmationStore.setState({ queue: [] });
    queue.forEach(request => {
        request.resolve(false);
    });
}

// Mount once at the application root, outside individual screens and cards.
export function ConfirmationModal() {
    useEffect(() => cancelConfirmations, []);
    const request = useConfirmationStore(state => state.queue[0]);
    if (!request) return null;

    const answer = (confirmed: boolean) => {
        // Ignore a repeated click on a dialog that has already been answered.
        if (useConfirmationStore.getState().queue[0] !== request) return;
        useConfirmationStore.setState(state => ({ queue: state.queue.slice(1) }));
        request.resolve(confirmed);
    };

    return <YesNoModal
        isVisible
        title={request.title}
        onYes={() => answer(true)}
        onNo={() => answer(false)}>
        {request.children}
    </YesNoModal>;
}
