@echo off
rem Optional fallback: PLC Lab starts by double-clicking "PLC Lab.exe" in this folder.
rem Development mode is RUN_PLC_LAB_DEV.bat.
setlocal EnableExtensions DisableDelayedExpansion
if not exist "%~dp0PLC Lab.exe" (
  echo PLC Lab.exe was not found next to this file. Run BUILD_PLC_LAB_RELEASE.bat first.
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
start "" "%~dp0PLC Lab.exe"
if errorlevel 1 (
  echo ERROR: "%~dp0PLC Lab.exe" could not be started.
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b 1
)
exit /b 0
