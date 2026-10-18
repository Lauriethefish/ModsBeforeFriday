#!/usr/bin/env pwsh
# Cargo-watch must be installed to use this script.
# To install it, run `cargo install cargo-watch`

Write-Output "Waiting for agent modifications"
$AgentSource = Join-Path $PSScriptRoot "mbf-agent/src"
$ResourceManagerSource = Join-Path $PSScriptRoot "mbf-res-man/src"
$BuildScript = Join-Path $PSScriptRoot "build_agent.ps1"
cargo watch -w $AgentSource -w $ResourceManagerSource -s "pwsh -File `"$BuildScript`""
