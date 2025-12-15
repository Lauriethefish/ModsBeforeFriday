import { Eng } from "../localization/en";
import { getLang, setLang, EngLikeObject } from "../localization/shared";
import { SimplifiedChinese } from "../localization/zh_cn";

function LangButton({lang, children}: {
    lang: EngLikeObject, 
    children: any
}){
    return <button className={"langBtn" +(getLang() as any == lang ? " selectedLangBtn":"")} onClick={() => setLang(lang)}>{children}</button>
}

export function LanguagePicker() {
    return <>
        <div className="langBtnContainer">
            <LangButton lang={Eng}>En</LangButton>
            <LangButton lang={SimplifiedChinese}>中文</LangButton>
        </div>
    </>
}