# ModsBeforeFriday

[ModsBeforeFriday](https://lauriethefish.github.io/ModsBeforeFriday/) is a modding tool for Beat Saber on Quest that works entirely within the browser, using WebUSB to interact with the quest. The aim is to make installing mods as easy as possible, with no need to download special tools or hunt around for core mods.

## Project Structure

`./mbf-agent` contains the agent, which is an executable written in Rust that is executed by the frontend via ADB. This agent does pretty much all the work, including installing mods and patching the game.
`./mbf-site` contains the frontend, which communicates with the agent via JSON. (Written in typescript with React).

## Compilation Instructions
### Build Requirements
- [yarn 1.22](https://classic.yarnpkg.com/lang/en/docs/install/)
- Rust 1.77 or newer, install with [rustup](https://rustup.rs/).
- [Android NDK](https://developer.android.com/ndk/downloads), r23b or newer.

### Setting up Rust
Install the aarch64-linux-android target:

```$ rustup target add aarch64-linux-android```

#### Environment Variables
- Set `ANDROID_NDK_HOME` to the folder containing your Android NDK.
- Set `CC_aarch64-linux-android` to `$NDK_PATH/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android31-clang.cmd` where `$NDK_PATH` is your Android NDK root path.
- Set `AR_aarch64-linux-android` to `$NDK_PATH/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar.exe`.

(if on another OS, the paths may be slightly different. Please update the paths as necessary!)
#### Cargo config
Create a new file with path `~/.cargo/config.toml`. Add the following contents, replacing the `<contents of...>` with the relevant environment variable.
```toml
[target.aarch64-linux-android]
linker = "<contents OF CC_aarch64-linux-android environment variable>"
ar = "<contents OF AR_aarch64-linux-android environment variable>"
```

### Compiling Agent
- To compile the agent and copy it to the `public` directory so that it can be used by the site, navigate to `./mbf-agent` and run `./build.ps1`.
- The `./run_android` script can be used to automatically copy the agent to the correct location on the Quest if you want to invoke it manually in `adb shell`. The site will do this automatically otherwise.
- `./reset_bs` will reinstall vanilla Beat Saber. (The paths in this file reflect the directory structure of Lauriethefish's computer and will need updating with the path of your APK/OBB.)

### Debugging site
To serve the site for testing, navigate to `./mbf-site` and run `yarn start`.
(you may need to `yarn install` first).