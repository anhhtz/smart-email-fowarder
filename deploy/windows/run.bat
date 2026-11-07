@echo off
title Smart Email Forwarder
echo ===================================================
echo   Smart Email Forwarder - Offline Console Launcher
echo ===================================================
echo.

cd /d "%~dp0\..\.."

if exist "smart-email-forwarder.exe" (
    smart-email-forwarder.exe --port 8080 --host 0.0.0.0
) else if exist "target\release\smart-email-forwarder.exe" (
    target\release\smart-email-forwarder.exe --port 8080 --host 0.0.0.0
) else if exist "target\debug\smart-email-forwarder.exe" (
    target\debug\smart-email-forwarder.exe --port 8080 --host 0.0.0.0
) else (
    echo [ERROR] Executable smart-email-forwarder.exe not found.
    echo Please place smart-email-forwarder.exe in this directory.
    pause
)
