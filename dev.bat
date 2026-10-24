@echo off
rem Starts the Clarity app in development mode, from any directory.
rem
rem   dev.bat                          opens the project used last time
rem   dev.bat examples\shop.clarity    opens a project file
rem
rem The designer writes its changes into the project's files, so point it at
rem a copy when you only want to experiment.
setlocal

set "root=%~dp0"

if "%~1"=="" goto :start
if not exist "%~f1" goto :noproject
rem A directory is not a project file.
if exist "%~f1\" goto :noproject
rem Absolute, because the app runs from app\.
set "CLARITY_PROJECT=%~f1"
goto :start

:noproject
echo dev.bat: '%~1' is not a project file; give one like examples\shop.clarity 1>&2
exit /b 1

:start
cd /d "%root%app" || exit /b 1
if not exist node_modules (
    call npm install || exit /b 1
)
call npm run tauri dev
exit /b %errorlevel%
