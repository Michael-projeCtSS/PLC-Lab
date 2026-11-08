@echo off
rem Build the current commit of this checkout and install it as the root PLC Lab.exe
rem (UNSIGNED DEVELOPMENT BUILD). It keeps a backup of the previous version, installs
rem atomically, verifies every file and runs a native check before it reports success.
rem A signed production release is never replaced. Uncommitted work: use TEST_PLC_LAB.bat.
setlocal EnableExtensions DisableDelayedExpansion
echo UNSIGNED DEVELOPMENT BUILD - updates the root PLC Lab.exe from this checkout.
call "%~dp0scripts\human-launcher.bat" "%~dp0" update %*
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_EXIT%"=="0" (
  echo.
  echo PLC LAB UPDATE DID NOT COMPLETE - the messages above say why.
  echo The root PLC Lab.exe is unchanged or was restored. TEST_PLC_LAB.bat runs the current source.
  if not "%PLC_LAB_NO_PAUSE%"=="1" pause
  exit /b %PLC_LAB_EXIT%
)
if "%PLC_LAB_NO_PAUSE%"=="1" exit /b 0
choice /C YN /N /M "Start the updated PLC Lab now? [Y/N] "
if errorlevel 2 exit /b 0
start "" "%~dp0PLC Lab.exe"
exit /b 0
