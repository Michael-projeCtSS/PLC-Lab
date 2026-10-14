@echo off
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0scripts\human-launcher.bat" "%~dp0" dev
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_EXIT%
