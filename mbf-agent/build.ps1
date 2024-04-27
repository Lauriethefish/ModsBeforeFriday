$TARGET = "aarch64-linux-android"
$OutputDirectory = "$PSScriptRoot/../mbf-site/public"
$HashOutputPath = "$OutputDirectory/mbf-agent.sha1"
$OutputPath = "$OutputDirectory/mbf-agent"

cargo build --target $TARGET --release
Copy-Item $PSScriptRoot/target/$TARGET/release/mbf-agent $OutputPath
$SHA1Hash = Get-FileHash -Algorithm SHA1 -Path $PSScriptRoot/target/$TARGET/release/mbf-agent | select -ExpandProperty Hash
$Utf8NoBomEncoding = New-Object System.Text.UTF8Encoding $False
[System.IO.File]::WriteAllLines($HashOutputPath, $SHA1Hash, $Utf8NoBomEncoding)