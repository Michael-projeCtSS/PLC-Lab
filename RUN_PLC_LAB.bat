@echo off
rem Starts the staged PLC Lab release. Development mode is RUN_PLC_LAB_DEV.bat.
setlocal EnableExtensions DisableDelayedExpansion
set "PLC_LAB_DIR=%~dp0releases\current"
set "PLC_LAB_APP=%PLC_LAB_DIR%\PLC Lab Desktop.exe"
if not exist "%PLC_LAB_APP%" (
  echo No local PLC Lab release exists. Run BUILD_PLC_LAB_RELEASE.bat first.
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
start "" /D "%PLC_LAB_DIR%" "%PLC_LAB_APP%"
if errorlevel 1 (
  echo ERROR: PLC Lab could not be started from "%PLC_LAB_APP%".
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
exit /b 0
