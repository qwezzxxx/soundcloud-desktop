@echo off
setlocal
where gcc >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    if exist "C:\Users\games\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin" (
        set "PATH=C:\Users\games\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin;%PATH%"
    )
)
cargo build --manifest-path "%~dp0src-tauri\Cargo.toml"
if %ERRORLEVEL% EQU 0 (
    copy /y "%~dp0src-tauri\target\debug\soundcloud-desktop.exe" "%~dp0SoundCloud.exe"
    if exist "%~dp0src-tauri\target\debug\WebView2Loader.dll" (
        copy /y "%~dp0src-tauri\target\debug\WebView2Loader.dll" "%~dp0WebView2Loader.dll"
    )
    python "%~dp0embed_icon.py"
    echo [OK] Build successful: SoundCloud.exe updated.
) else (
    echo [ERROR] Cargo build failed!
    exit /b %ERRORLEVEL%
)
