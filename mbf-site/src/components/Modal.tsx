import { ReactNode } from "react";
import { ReactComponent as AlertIcon } from '../icons/alert-triangle.svg'

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
        return <></>
    }
}

interface ErrorModalProps {
    isVisible: boolean,
    title: string,
    description: string
}

export function ErrorModal(props: ErrorModalProps) {
    return <Modal isVisible={props.isVisible}>
        <div id="errorTitle">
            <AlertIcon fill="white"/>
            <h1>{props.title}</h1>
        </div>
        <p>{props.description}</p>
    </Modal>
}