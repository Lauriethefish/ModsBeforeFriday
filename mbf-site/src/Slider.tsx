import { useState } from "react";
import './Slider.css';

export interface SliderProps {
    on: boolean,
    valueChanged: (newValue: boolean) => void
}

export function Slider(props: SliderProps) {
    return <label className="switch">
        <input type="checkbox" checked={props.on} onClick={() => props.valueChanged(!props.on)} />
        <span className="slider"></span>
    </label>
}