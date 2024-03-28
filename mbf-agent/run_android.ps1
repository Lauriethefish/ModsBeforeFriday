$SkipReinstall=$args[0]

Write-Output "Building"
cargo build --release --target aarch64-linux-android
$PROGRAM = "./target/aarch64-linux-android/release/mbf-agent"
$QUEST_LOC = "/data/local/tmp/mbf-agent"
Write-Output "Pushing"
adb push $PROGRAM $QUEST_LOC
adb shell chmod +x $QUEST_LOC
if ( $SkipReinstall.Length -eq 0 ) {
    ./reset_bs
}
