Write-Output "Resetting to vanilla BS"
adb shell pm uninstall com.beatgames.beatsaber
adb install bs.apk
adb push "C:\Users\Lauri\Beat_Saber_Mod_Dev\main.1130.com.beatgames.beatsaber.obb" "/sdcard/Android/obb/com.beatgames.beatsaber/main.1130.com.beatgames.beatsaber.obb"