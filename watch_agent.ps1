# Cargo-watch must be installed to use this script.
# To install it, run `cargo install cargo-watch`

Write-Output "Waiting for agent modifications"
cargo watch -w "$PSScriptRoot\mbf-agent\src\" -w "$PSScriptRoot\mbf-res-man\src\" -s "powershell $PSScriptRoot/build_agent.ps1"