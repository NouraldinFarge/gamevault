@echo off
setlocal
title GameVault Portable Build

powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0BUILD-LATEST.ps1"
set "GAMEVAULT_BUILD_EXIT=%ERRORLEVEL%"

echo.
if not "%GAMEVAULT_NO_PAUSE%"=="1" pause
exit /b %GAMEVAULT_BUILD_EXIT%

