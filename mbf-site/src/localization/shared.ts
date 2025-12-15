import { useState } from "react";
import { Eng } from "./en"

interface EngImplementType {
    __proto__: EngImplementType | typeof Eng
}
export type EngLikeObject = EngImplementType | typeof Eng

let _lang: EngLikeObject = Eng
let _setLang: (lang: any) => void

export function initLanguage() {
    [_lang, _setLang] = useState(Eng)
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