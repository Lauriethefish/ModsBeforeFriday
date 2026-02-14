import { useState } from "react";
import { Eng } from "./en"
import { SimplifiedChinese } from "./zh_cn";

interface EngImplementType {
    __proto__: EngImplementType | typeof Eng
}
export type EngLikeObject = EngImplementType | typeof Eng

let _lang: EngLikeObject = Eng
let _setLang: (lang: any) => void

export function initLanguage() {
    const languageInUrl = new URLSearchParams(window.location.search).get("lang");
    let defaultLang:EngLikeObject = Eng
    
    if(languageInUrl == "zh_cn")
        defaultLang = SimplifiedChinese;

    [_lang, _setLang] = useState(defaultLang)
}

export function getLang(): typeof Eng {
    return _lang as typeof Eng
}

export function setLang(
    lang: EngLikeObject
) {
    if (_setLang)
        _setLang(lang)
}