import { getLangStr, setLangStr } from "../localization/shared";

export function LanguagePicker() {
    return <>
        <div>
            <button onClick={() => setLangStr("en")}>en</button>
            <button onClick={() => setLangStr("zh_cn")}>🇨🇳</button>
        </div>
    </>
}