import { useState } from "react";
import { Eng } from "./en"
import { ZhCn } from "./zh_cn";

const Languages: Record<string, any> = {
    "en": Eng,
    "zh_cn": ZhCn
}

class LanguageManager {
    lang: string
    setLang: (lang: any) => void
    static instance: LanguageManager
    constructor() {
        [this.lang, this.setLang] = useState("en")
    }
}

export function initializeLocalization() {
    LanguageManager.instance = new LanguageManager()
}


export function getLang(): typeof Eng {
    return Languages[LanguageManager.instance?.lang ?? ""] ?? Eng // the element inside the Language act as typeof Eng
}

export function getLangStr() {
    return LanguageManager.instance.lang
}

export function setLangStr(lang: string) {
    if (LanguageManager.instance) {
        LanguageManager.instance.setLang(lang)
    }
}