import { useState } from "react";
import { Eng } from "./en"

let _lang:typeof Eng = Eng
let _setLang:any

export function initLanguage() {
    [_lang, _setLang] = useState(Eng)
}

export function getLang(): typeof Eng {
    return _lang
}

export function setLang(
    lang: any // lang is type of Eng, or act as Eng. This is duck-typed, and provided by other languages
) {
    if(_setLang)
        _setLang(lang)
}