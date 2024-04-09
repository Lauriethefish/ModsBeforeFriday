# Put your OBB file/BS APK here
if ( $Latest.Length -eq 0 ) {
    Write-Output "Using latest: 1.36"
    $OBB_PATH = "main.1188.com.beatgames.beatsaber.obb"
    $BS_PATH = "bs136.apk"
}   else    {
    $OBB_PATH = "main.1130.com.beatgames.beatsaber.obb"
    $BS_PATH = "bs135.apk"
}

Write-Output "Resetting to vanilla BS"
adb shell pm uninstall com.beatgames.beatsaber
adb install $BS_PATH
adb push $OBB_PATH "/sdcard/Android/obb/com.beatgames.beatsaber/$OBB_PATH"
adb kill-server