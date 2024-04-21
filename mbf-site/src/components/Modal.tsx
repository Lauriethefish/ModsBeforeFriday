import { ReactNode } from "react";
import AlertIcon from '../icons/alert-triangle.svg'

import '../css/Modal.css';
import { LogMsg } from "../Messages";
import { LogWindow } from "./LogWindow";

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
            <button onClick={props.onYes}>Yes</button>
            <button onClick={props.onNo}>No</button>
        </div>
    </Modal>
}

interface SyncingModalProps {
    isVisible: boolean,
    title: string,
    logEvents: LogMsg[]
}

export function SyncingModal(props: SyncingModalProps) {
    if(props.isVisible) {
        return  <div className="modalBackground">
            <div className="modal container screenWidth">
                <div className="syncingWindow">
                    <h1>{props.title}</h1>
                    <LogWindow events={props.logEvents} />
                </div>
            </div>
        </div>
    }   else   {
        return <div className="modalBackground modalClosed"></div>
    }
}