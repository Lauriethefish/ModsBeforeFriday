import { ReactNode } from "react";
import '../css/Modal.css';

interface ModalProps {
    children: ReactNode,
    isVisible: boolean
}

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