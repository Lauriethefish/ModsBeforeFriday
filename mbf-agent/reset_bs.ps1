$Latest=$args[0]
$APKS_PATH = "./apk_data"

# Put your OBB file/BS APK here
if ( $Latest.Length -ne 0 ) {
    Write-Output "Using latest: 1.37.0"
	$OBB_NAME = "main.1238.com.beatgames.beatsaber.obb"
    $BS_PATH = "$APKS_PATH/bs137.apk"
}   else    {
    $OBB_NAME = "main.1130.com.beatgames.beatsaber.obb"
    $BS_PATH = "$APKS_PATH/bs135.apk"
}

$OBB_PATH = "$APKS_PATH/$OBB_NAME"

Write-Output "Resetting to vanilla BS"
adb shell pm uninstall com.beatgames.beatsaber
adb install $BS_PATH
adb push $OBB_PATH "/sdcard/Android/obb/com.beatgames.beatsaber/$OBB_NAME"
adb kill-server