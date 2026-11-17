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
rem Apps generated and built here use the runtimes of this repository, not the
rem published ones, so a change to Clarity.UI shows in the next Run. C# wants
rem the C# runtime's folder; Rust finds its runtime beside it.
if not defined CLARITY_RUNTIME set "CLARITY_RUNTIME=%root%runtimes\csharp"

cd /d "%root%app" || exit /b 1
rem Installs again whenever package-lock.json changed since the last install,
rem not only when node_modules is missing: a package added later (@clarity/ui,
rem linked from runtimes\web) is otherwise never there ("Failed to resolve import").
fc /b package-lock.json node_modules\.clarity-installed-lock.json >nul 2>&1 || (
    call npm install || exit /b 1
    copy /y package-lock.json node_modules\.clarity-installed-lock.json >nul
)
rem The folders Clarity settings > DevTools > Disk marked for the next start
rem go now, before anything builds or holds their files (a running IDE holds
rem its own on Windows, so Remove cannot take them).
node scripts\clean-at-start.mjs
rem Vite's cache (node_modules\.vite) is kept, as dev.sh keeps it.
call npm run tauri dev
exit /b %errorlevel%
