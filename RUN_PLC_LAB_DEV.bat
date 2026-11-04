@echo off
rem Native PLC Lab in Tauri development mode (real Rust backend). TEST_PLC_LAB.bat does the
rem same and also keeps a log file of the run.
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0scripts\human-launcher.bat" "%~dp0" dev
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_EXIT%"=="0" (
  echo.
  echo PLC LAB DEVELOPMENT START FAILED - the messages above say why.
)
if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_EXIT%
