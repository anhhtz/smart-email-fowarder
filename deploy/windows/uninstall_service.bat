@echo off
title Uninstall Smart Email Forwarder Service

echo ========================================================
echo   Smart Email Forwarder - Windows Service Uninstaller
echo ========================================================
echo.

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [ERROR] Administrative permissions required.
    echo Please right-click this batch file and select "Run as administrator".
    pause
    exit /b 1
)

set "SERVICE_NAME=SmartEmailForwarder"

echo Stopping service '%SERVICE_NAME%'...
sc.exe stop "%SERVICE_NAME%"

echo Removing service '%SERVICE_NAME%'...
sc.exe delete "%SERVICE_NAME%"

echo Service '%SERVICE_NAME%' uninstalled successfully.
pause
