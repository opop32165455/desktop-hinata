@echo off
REM ============================================================
REM  Hinata - Windows one-click build (Tauri / NSIS installer)
REM ------------------------------------------------------------
REM  Prereqs : Rust (stable-x86_64-pc-windows-msvc)
REM            Visual Studio 2022 Build Tools (C++ workload)
REM            Node.js + `npm install` already run once in this folder
REM  Usage   : build-windows.bat            -> full build (media included)
REM            build-windows.bat nomedia    -> update build: no media, small + fast
REM            build-windows.bat check      -> only check the env
REM  Output  : src-tauri\target\release\bundle\nsis\*setup.exe
REM
REM  The "nomedia" flavour skips the ~1GB of video (see
REM  src-tauri\tauri.nomedia.conf.json). Use it to update everything except
REM  media on a machine that already has a full install: the installer stays
REM  tiny, builds in a fraction of the time, and leaves the existing media\
REM  folder untouched. It is NOT a standalone installer - media must already
REM  be present in the install directory.
REM ============================================================
setlocal

cd /d "%~dp0"

REM ---------- 0. Build flavour ----------
set "NOMEDIA="
if /i "%~1"=="nomedia" set "NOMEDIA=1"

set "PF86=%ProgramFiles(x86)%"
if not defined PF86 set "PF86=C:\Program Files (x86)"

REM ---------- 1. Locate Visual Studio Build Tools ----------
set "VSWHERE=%PF86%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo [ERROR] vswhere.exe not found. Install "Visual Studio 2022 Build Tools".
  exit /b 1
)
set "VSPATH="
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSPATH=%%i"
if not defined VSPATH (
  echo [ERROR] MSVC C++ build tools not found. Add the "Desktop development with C++" workload.
  exit /b 1
)

REM ---------- 2. Newest MSVC toolset ----------
set "MSVCVER="
for /f "delims=" %%i in ('dir /b /ad /o-n "%VSPATH%\VC\Tools\MSVC" 2^>nul') do if not defined MSVCVER set "MSVCVER=%%i"

REM ---------- 3. Newest Windows 10/11 SDK ----------
set "SDKROOT=%PF86%\Windows Kits\10"
set "SDKVER="
for /f "delims=" %%i in ('dir /b /ad /o-n "%SDKROOT%\Include" 2^>nul ^| findstr /r "^10\."') do if not defined SDKVER set "SDKVER=%%i"

REM ---------- 4. Locate cargo / node / project-local tauri CLI ----------
set "CARGOBIN=%USERPROFILE%\.cargo\bin"
set "TAURI=%CD%\node_modules\.bin\tauri.cmd"

echo.
echo   VS path      : %VSPATH%
echo   MSVC toolset : %MSVCVER%
echo   Windows SDK  : %SDKVER%
echo   cargo        : %CARGOBIN%
echo   tauri CLI    : %TAURI%
echo.

if not exist "%VSPATH%\VC\Tools\MSVC\%MSVCVER%\bin\Hostx64\x64\link.exe" (
  echo [ERROR] link.exe not found for toolset "%MSVCVER%".
  exit /b 1
)
if not exist "%CARGOBIN%\cargo.exe" (
  echo [ERROR] cargo.exe not found at "%CARGOBIN%".
  exit /b 1
)
if not exist "%TAURI%" (
  echo [ERROR] Local tauri CLI not found. Run "npm install" in this folder first.
  exit /b 1
)
where node >nul 2>nul
if errorlevel 1 (
  echo [ERROR] node.exe not found on PATH.
  exit /b 1
)

if /i "%~1"=="check" (
  echo [OK] Environment looks good.
  exit /b 0
)

REM ---------- 5. Build environment ----------
REM  rustc does NOT set MSVC paths up on its own, and Git Bash's GNU
REM  link.exe would otherwise shadow the MSVC linker.
set "PATH=%VSPATH%\VC\Tools\MSVC\%MSVCVER%\bin\Hostx64\x64;%SDKROOT%\bin\%SDKVER%\x64;%CARGOBIN%;%PATH%"
set "LIB=%VSPATH%\VC\Tools\MSVC\%MSVCVER%\lib\x64;%SDKROOT%\Lib\%SDKVER%\ucrt\x64;%SDKROOT%\Lib\%SDKVER%\um\x64"
set "INCLUDE=%VSPATH%\VC\Tools\MSVC\%MSVCVER%\include;%SDKROOT%\Include\%SDKVER%\ucrt;%SDKROOT%\Include\%SDKVER%\um;%SDKROOT%\Include\%SDKVER%\shared"

REM ---------- 6. Build ----------
set "NSISDIR=%~dp0src-tauri\target\release\bundle\nsis"
set "TAURI_ARGS="

if defined NOMEDIA (
  REM  Both flavours write the same output filename. Set any existing full
  REM  installer aside first, otherwise this build would clobber it.
  for %%f in ("%NSISDIR%\Hinata_*_x64-setup.exe") do (
    move /y "%%f" "%NSISDIR%\Hinata-full-setup.exe" >nul
  )
  set "TAURI_ARGS=--config src-tauri\tauri.nomedia.conf.json"
  echo Flavour      : nomedia ^(no media, update only^)
) else (
  echo Flavour      : full ^(media included^)
)
echo Building... (first run ~15-20 min, later runs faster)
echo.
call "%TAURI%" build %TAURI_ARGS%
if errorlevel 1 (
  echo.
  echo [FAILED] Build failed. See the output above.
  exit /b 1
)

echo.
if defined NOMEDIA (
  REM  Give the media-less installer an obvious name so it can never be
  REM  mistaken for the full one.
  for %%f in ("%NSISDIR%\Hinata_*_x64-setup.exe") do (
    move /y "%%f" "%NSISDIR%\Hinata-update-setup.exe" >nul
  )
)

echo ============================================================
echo  [OK] Installer produced:
echo ============================================================
dir /b "%NSISDIR%\*.exe"
echo.
if defined NOMEDIA (
  echo  Hinata-update-setup.exe : this build. Install it OVER an existing full
  echo                           install. It carries no media, and the media
  echo                           folder already on disk is left untouched.
  echo  Hinata-full-setup.exe   : the full installer, if one was present before.
  echo.
)
endlocal
