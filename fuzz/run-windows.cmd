@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "RUNS=%~1"
if not defined RUNS set "RUNS=1000"

for /f "usebackq delims=" %%I in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.ASAN -property installationPath`) do set "VS_PATH=%%I"

if not defined VS_PATH (
  echo Visual Studio C++ AddressSanitizer is not installed. 1>&2
  exit /b 1
)

for /f "delims=" %%I in ('where /r "!VS_PATH!\VC\Tools\MSVC" clang_rt.asan_dynamic-x86_64.dll ^| findstr /i "\\bin\\Hostx64\\x64\\"') do set "ASAN_PATH=%%~dpI"

if not defined ASAN_PATH (
  echo The x64 AddressSanitizer runtime DLL was not found. 1>&2
  exit /b 1
)

set "PATH=!ASAN_PATH!;!PATH!"
cargo +nightly fuzz run decoder_boundaries -- -runs=!RUNS!
exit /b !ERRORLEVEL!
