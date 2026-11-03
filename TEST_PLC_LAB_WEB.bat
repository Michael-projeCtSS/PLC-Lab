@echo off
setlocal EnableExtensions DisableDelayedExpansion
pushd "%~dp0"
if errorlevel 1 (
  echo ERROR: Cannot open the PLC Lab repository directory.
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
"%SystemRoot%\System32\where.exe" node.exe >nul 2>nul
if errorlevel 1 (
  echo ERROR: Node.js is missing. Install Node.js 24 or later with npm.
  popd
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
if "%~1"=="--no-open" (
  node "scripts\web-preview.mjs" --no-open
) else (
  node "scripts\web-preview.mjs"
)
set "PLC_LAB_WEB_EXIT=%ERRORLEVEL%"
popd
if not "%PLC_LAB_WEB_EXIT%"=="0" if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_WEB_EXIT%
