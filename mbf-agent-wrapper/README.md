# Agent Wrapper

This folder contains `mbf-agent-wrapper.py`, which is a Python (3.9.0 or newer) script that provides a command line interface for interacting with the ModsBeforeFriday backend. This allows the use of MBF without a chromium browser installed, and the script uses a regular ADB client rather than WebUSB so there's no need to manually kill any existing ADB servers.

To use the agent wrapper, ensure you have `adb` on PATH, then run `python mbf-agent-wrapper.py -h` for further details of usage.
By running `python mbf-agent-wrapper.py Interactive`, you can enter multiple commands without having to invoke the script separately each time.