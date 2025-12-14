import { Eng } from "../localization/en";
import { setLang } from "../localization/shared";
import { SimplifiedChinese } from "../localization/zh_cn";

export function LanguagePicker() {
    return <>
        <div className="langBtnContainer">
            <a href="javascript:void(0)" className="langBtn" onClick={() => setLang(Eng)}>En</a>
            <a href="javascript:void(0)" className="langBtn" onClick={() => setLang(SimplifiedChinese)}>中文(制作中)</a>
        </div>
    </>
}