export function usingOculusBrowser(): boolean {
    return navigator.userAgent.includes("OculusBrowser");
}
  
export function isViewingOnWindows(): boolean {
    // Deprecated but still works for our purposes.
    return navigator.appVersion.indexOf("Win") != -1;
}

export function isViewingOnMobile() {
    return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}

// Kindly provided by Pierre
// https://stackoverflow.com/a/9039885
export function isViewingOnIos() {
    return [
        'iPad Simulator',
        'iPhone Simulator',
        'iPod Simulator',
        'iPad',
        'iPhone',
        'iPod'
        // This is deprecated but still provides a good way to detect iOS as far as the author is concerned.
        // We are also doing feature detection for WebUSB, but detecting iOS provides a good way to warn the user that no iOS browsers will work with this app.
    ].includes(navigator.platform)
    // iPad on iOS 13 detection
    || (navigator.userAgent.includes("Mac") && "ontouchend" in document)
}