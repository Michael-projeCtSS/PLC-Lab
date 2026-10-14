@echo off
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0scripts\human-launcher.bat" "%~dp0" build-release
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_EXIT%"=="0" echo RELEASE BUILD FAILED - read the error output above.
if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_EXIT%
