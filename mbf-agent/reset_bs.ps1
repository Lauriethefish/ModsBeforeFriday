$Latest=$args[0]
$APKS_PATH = "./apk_data"

# Put your OBB file/BS APK here
if ( $Latest.Length -ne 0 ) {
    Write-Output "Using latest: 1.36.1"
    $OBB_PATH = "$APKS_PATH/main.1194.com.beatgames.beatsaber.obb"
    $BS_PATH = "$APKS_PATH/bs1361.apk"
}   else    {
    $OBB_PATH = "$APKS_PATH/main.1130.com.beatgames.beatsaber.obb"
    $BS_PATH = "$APKS_PATH/bs135.apk"
}

Write-Output "Resetting to vanilla BS"
adb shell pm uninstall com.beatgames.beatsaber
adb install $BS_PATH
adb push $OBB_PATH "/sdcard/Android/obb/com.beatgames.beatsaber/$OBB_PATH"
adb kill-server