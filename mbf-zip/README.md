# MBF ZIP Library
This library crate allows the reading and writing of files in ZIP format.

The ZIP editor appends any added files directly to the end of the ZIP, and will not remove the content of removed files
from the underlying ZIP. (it is indended to be a minimal implementation).

This library also supports the signing of ZIP files with the APK signature scheme v2 so that they can be installed by the Quest (2/3/Pro).
