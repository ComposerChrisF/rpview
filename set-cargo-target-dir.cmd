@echo off
REM Sets CARGO_TARGET_DIR as a persistent user environment variable.
REM This keeps Rust build artifacts on the local Windows disk,
REM avoiding path-length limits and slow network-drive I/O when
REM building from a mapped Parallels drive.
REM
REM Run this once from any command prompt (no admin required),
REM or use it as a reference to set the variable manually via
REM System Properties > Environment Variables > User variables.

setx CARGO_TARGET_DIR "C:\rust-target\rpview-gpui"

echo.
echo CARGO_TARGET_DIR has been set to C:\rust-target\rpview-gpui
echo Close and reopen your terminal for the change to take effect.
