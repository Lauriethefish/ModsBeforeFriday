# Put your OBB file/BS APK here
$OBB_PATH = "C:\Users\Lauri\Beat_Saber_Mod_Dev\main.1130.com.beatgames.beatsaber.obb"
$BS_PATH = "bs.apk"

Write-Output "Resetting to vanilla BS"
adb shell pm uninstall com.beatgames.beatsaber
adb install bs.apk
adb push $OBB_PATH "/sdcard/Android/obb/com.beatgames.beatsaber/main.1130.com.beatgames.beatsaber.obb"
adb kill-server