@echo off
setlocal EnableExtensions DisableDelayedExpansion
call "%~dp0scripts\human-launcher.bat" "%~dp0" checks
set "PLC_LAB_EXIT=%ERRORLEVEL%"
echo.
if "%PLC_LAB_EXIT%"=="0" (echo ALL CHECKS PASSED) else (echo CHECKS FAILED)
if not "%PLC_LAB_NO_PAUSE%"=="1" pause
exit /b %PLC_LAB_EXIT%
