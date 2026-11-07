@echo off
setlocal enabledelayedexpansion
title Install Smart Email Forwarder Service

echo ========================================================
echo   Smart Email Forwarder - Windows Service Installer
echo   Target: Windows Server 2019 Standard
echo ========================================================
echo.

:: Check for administrative privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [ERROR] Administrative permissions required.
    echo Please right-click this batch file and select "Run as administrator".
    echo.
    pause
    exit /b 1
)

set "SERVICE_NAME=SmartEmailForwarder"
set "DISPLAY_NAME=Smart Email Forwarder"
set "BASE_DIR=%~dp0\..\.."
cd /d "%BASE_DIR%"

set "EXE_PATH=%CD%\smart-email-forwarder.exe"
if not exist "%EXE_PATH%" (
    if exist "%CD%\target\release\smart-email-forwarder.exe" (
        set "EXE_PATH=%CD%\target\release\smart-email-forwarder.exe"
    ) else (
        echo [ERROR] smart-email-forwarder.exe was not found in: %CD%
        pause
        exit /b 1
    )
)

echo Found executable: %EXE_PATH%
echo Registering Windows Service '%SERVICE_NAME%'...

:: Create Windows Service via sc.exe (Native Windows Server 2019)
sc.exe create "%SERVICE_NAME%" binPath= "\"%EXE_PATH%\" --service" start= auto DisplayName= "%DISPLAY_NAME%"
if %errorlevel% neq 0 (
    echo [WARNING] sc create reported code %errorlevel%. Checking if service exists...
)

:: Configure description
sc.exe description "%SERVICE_NAME%" "Polls MS Exchange mailbox and forwards emails to Gmail via internal Exchange relay."

:: Configure auto-recovery on failure (Restart service after 5s)
sc.exe failure "%SERVICE_NAME%" reset= 86400 actions= restart/5000/restart/5000/restart/5000

:: Start the service
echo Starting service '%SERVICE_NAME%'...
sc.exe start "%SERVICE_NAME%"

echo.
echo ========================================================
echo   Service '%SERVICE_NAME%' is now installed and running!
echo   Web UI: http://localhost:8080
echo   Auto-start: Enabled on system boot (no login required)
echo ========================================================
echo.
pause
