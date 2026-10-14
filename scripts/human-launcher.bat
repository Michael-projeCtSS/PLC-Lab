@echo off
setlocal EnableExtensions DisableDelayedExpansion
echo ============================================================
echo PLC Lab human testing: %~2
echo Repository: %~1
echo ============================================================
pushd "%~1"
if errorlevel 1 (
  echo ERROR: Cannot open the repository directory.
  exit /b 1
)
if "%~2"=="release" goto release
where node.exe >nul 2>nul
if errorlevel 1 (
  echo ERROR: Node.js is missing. Install Node.js 24 or later with npm.
  echo See docs\HUMAN_TESTING.md for first-time prerequisites.
  popd
  exit /b 1
)
node "scripts\human-test.mjs" "%~2"
set "PLC_LAB_EXIT=%ERRORLEVEL%"
popd
exit /b %PLC_LAB_EXIT%

:release
if not exist "releases\current\PLC Lab.exe" (
  echo No local PLC Lab release exists. Run BUILD_PLC_LAB_RELEASE.bat first.
  popd
  exit /b 1
)
echo [RELEASE] Launching "%CD%\releases\current\PLC Lab.exe"
echo This uses the last successful local release, not the current source tree.
echo Close the application to finish this launcher.
start "" /wait "%CD%\releases\current\PLC Lab.exe"
set "PLC_LAB_EXIT=%ERRORLEVEL%"
if not "%PLC_LAB_EXIT%"=="0" echo ERROR: Release application returned exit code %PLC_LAB_EXIT%.
popd
exit /b %PLC_LAB_EXIT%
