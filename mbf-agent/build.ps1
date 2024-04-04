$TARGET = "aarch64-linux-android"
cargo build --target $TARGET --release
Copy-Item $PSScriptRoot/target/$TARGET/release/mbf-agent $PSScriptRoot/../mbf-site/public/mbf-agent
