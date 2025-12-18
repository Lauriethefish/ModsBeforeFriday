import { useRef } from "react";
import { getLang } from "../localization/shared";

export function SplashScreenSelector({ selected, onSelected }:
    {
        selected: File | null,
        onSelected: (nowSelected: File | null) => void,
    }
) {
    const inputFile = useRef<HTMLInputElement | null>(null);

    return <>
        <button onClick={() => inputFile.current?.click()} className="discreetButton floatRight">
            <input type="file"
                accept=".png"
                id="file"
                multiple={false}
                ref={inputFile}
                style={{display: 'none'}}
                onChange={async ev => {
                    const files = ev.target.files;
                    if(files !== null) {
                        onSelected(files[0]);
                    }
                    ev.target.value = "";
                }}
            />{getLang().selectSplashScreen}
        </button>
        {selected !== null && <span className="hoverStrikethrough" style={{ fontSize: "small" }} onClick={() => onSelected(null)}>
<br/>{getLang().usingSplash(selected.name)}</span>}
        <br/>
    </>
}