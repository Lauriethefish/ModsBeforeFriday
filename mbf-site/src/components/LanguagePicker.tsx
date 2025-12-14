import { getLangStr, setLangStr } from "../localization/shared";

export function LanguagePicker() {
    return <>
        <div className="langBtnContainer">
            <a href="javascript:void(0)" className="langBtn" onClick={() => setLangStr("en")}>En</a>
            <a href="javascript:void(0)" className="langBtn" onClick={() => setLangStr("zh_cn")}>中文</a>
        </div>
    </>
}