# This page is the ModsBeforeFriday source code
# [CLICK HERE FOR THE MODDING APP](https://lauriethefish.github.io/ModsBeforeFriday/)

[ModsBeforeFriday](https://lauriethefish.github.io/ModsBeforeFriday/) is a modding tool for Beat Saber on Quest that works entirely within the browser, using WebUSB to interact with the quest. The aim is to make installing mods as easy as possible, with no need to download special tools or hunt around for core mods.

## Query Parameters
MBF has some query parameters which can be passed with the URL. These are useful for mod developers when testing core mods before they are officially released.
- `?dev=true`: This will override the normal version check, and always prompt the user to patch the currently installed Beat Saber game. NOTE: If you are not a mod developer, this **will not help you.** All it does is allows the modloader to be installed, it does not magically make the new version support mods and using this will only prevent you from downgrading Beat Saber. This is not "get mods only".
- `?setcores=prompt`: This will prompt the user to enter an alternative core mods URL to use to test that the core mod JSON is ready for release. This URL will then be stored in the query parameter for future page refreshes.

## Project Structure

- `./mbf-agent-core` contains the agent core written in Rust. This agent does pretty much all the work, including installing mods and patching the game.
- `./mbf-agent-runnable` contains the runnable agent binary, installed on the Quest, which is an executable that is executed by the frontend via ADB. Handles almost all work required through stdin/stdout.
- `./mbf-agent-wrapper` is a Python script that can be used to invoke the MBF backend with a command-line-interface, handy for developers or Chromium-haters.
- `./mbf-adb-killer` is a development utility that kills any running ADB server when the frontend tries to connect to your Quest, thus avoiding conflicts between MBF and other apps *during development only.*. 
- `./mbf-res-man` contains the MBF resource management project, which contains code used by MBF to access external resources e.g. core mods, but also for updating its own resource repositories, e.g. [MBF Diffs](https://github.com/Lauriethefish/mbf-diffs/releases) whenever a new version of Beat Saber is released.
- `./mbf-zip` is a simple library for reading/writing ZIP files (and signing APKs) used by the `mbf-agent`.
- `./mbf-site` contains the frontend, which communicates with the agent via JSON. (Written in typescript with React).

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
- To compile the agent and copy it to the `public` directory so that it can be used by the site, run `./build_agent.ps1`.

### Debugging site
To serve the site for testing, navigate to `./mbf-site` and run `yarn start`.
(you may need to `yarn install` first).