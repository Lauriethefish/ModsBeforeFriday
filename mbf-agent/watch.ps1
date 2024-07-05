# Cargo-watch must be installed to use this script.
# To install it, run `cargo install cargo-watch`

Write-Output "Waiting for agent modifications"
cargo watch -w "$PSScriptRoot\src\" -s "powershell $PSScriptRoot/build.ps1"