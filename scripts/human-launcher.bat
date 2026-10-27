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
