# MBF Resource Management

## Library
This Rust project is generally responsible for managing all external files that the MBF agent needs to access.
It is used as a library by `mbf-agent` to:
- Fetch and process data about available core mods.
- Fetch diff/patch files for downgrading.
- Fetch versions of `AndroidManifest.xml` for past Beat Saber versions.
- Fetch unstripped versions of `libunity.so` for use during patching.

## CLI
This project also contains a command line tool that can be used to manage the files in the `mbf-diffs` and `mbf-manifests` repositories.
This automates the process of diff generation and manifest extraction when a new Beat Saber update is released to the greatest extent possible.

Usage for the CLI can be obtained by executing `cargo run --release -- --help` in the project directory.
Please note: To upload files to releases, a valid github token must be contained within the text file `./GITHUB_TOKEN.txt`.