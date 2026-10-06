# Optional retail test data

Copy a private V2000 installation into this folder. Tests need `PRELOAD.DAT`
and the complete `Overlay/` directory (212 files). Original executables, videos
and other installation files may remain alongside them. Everything here except
this setup note is Git-ignored.

With only this note (or an empty folder), retail-dependent tests are reported as
ignored. Once any game files are present, incomplete or corrupt data fails the
tests. Do not add generated test output here.

To use an existing installation elsewhere, set `V2K_RETAIL_DIR` instead.
Relative paths are resolved from the repository root. See
[building and testing](../README.md#building) for commands.
