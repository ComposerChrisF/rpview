@echo off
REM Windows environment setup for building rpview with GPUI.
REM
REM Run this once from any command prompt (no admin required),
REM or use it as a reference to set the variables manually via
REM System Properties > Environment Variables > User variables.
REM
REM Close and reopen your terminal after running for changes to take effect.

REM Keep Rust build artifacts on the local Windows disk,
REM avoiding path-length limits and slow network-drive I/O
REM when building from a mapped Parallels drive.
setx CARGO_TARGET_DIR "C:\rust-target\rpview-gpui"

REM GPUI release builds need fxc.exe (DirectX shader compiler).
REM Set GPUI_FXC_PATH explicitly because where.exe can return
REM multiple matches, and GPUI's build.rs doesn't handle that.
REM Adjust the SDK version (10.0.22621.0) if yours differs.
setx GPUI_FXC_PATH "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\arm64\fxc.exe"

echo.
echo Environment variables set:
echo   CARGO_TARGET_DIR = C:\rust-target\rpview-gpui
echo   GPUI_FXC_PATH    = ...Windows Kits\10\bin\10.0.22621.0\arm64\fxc.exe
echo.
echo Close and reopen your terminal for changes to take effect.
