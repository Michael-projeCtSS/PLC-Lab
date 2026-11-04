@echo off
rem Native development test of PLC Lab: the real Tauri application with its Rust backend,
rem built from this checkout. No browser, no signing certificate. Close the window to finish.
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0scripts\human-launcher.bat" "%~dp0" test
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_EXIT%"=="0" (
  echo.
  echo PLC LAB TEST FAILED - the messages above say why.
  echo The full output is in "%~dp0artifacts\test-plc-lab\last-run.log".
)
if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_EXIT%
