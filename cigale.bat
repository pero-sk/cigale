@echo off
setlocal enabledelayedexpansion

set REPO_URL=https://github.com/pero-sk/cigale
set INSTALL_DIR=%USERPROFILE%\.cigale
set BIN_DIR=%INSTALL_DIR%\bin
set SRC_DIR=%INSTALL_DIR%\src

if /i "%~1"=="install" goto install
if /i "%~1"=="update" goto update
if /i "%~1"=="uninstall" goto uninstall
goto help

:check_deps
    echo Checking dependencies...
    where git >nul 2>&1
    if %errorlevel% neq 0 (
        echo Error: git is required but not installed
        echo Install from https://git-scm.com
        exit /b 1
    )
    where cargo >nul 2>&1
    if %errorlevel% neq 0 (
        echo Error: cargo is required but not installed
        echo Install Rust from https://rustup.rs
        exit /b 1
    )
    echo [OK] git found
    echo [OK] cargo found
    exit /b 0

:add_to_path
    for /f "tokens=2*" %%a in ('reg query "HKCU\Environment" /v PATH 2^>nul') do set "CURRENT_PATH=%%b"
    echo !CURRENT_PATH! | findstr /i "%BIN_DIR%" >nul
    if %errorlevel% equ 0 (
        echo PATH already contains %BIN_DIR%
    ) else (
        setx PATH "!CURRENT_PATH!;%BIN_DIR%"
        echo [OK] Added %BIN_DIR% to PATH
    )
    exit /b 0

:copy_binaries
    echo Installing binaries...

    copy /y "%SRC_DIR%\target\release\cigale_stdl.exe"   "%BIN_DIR%\cigale_stdl.exe"   >nul
    if %errorlevel% neq 0 ( echo Failed to copy cigale_stdl.exe! & exit /b 1 )

    copy /y "%SRC_DIR%\target\release\cigale_nostdl.exe" "%BIN_DIR%\cigale_nostdl.exe" >nul
    if %errorlevel% neq 0 ( echo Failed to copy cigale_nostdl.exe! & exit /b 1 )

    :: copy bat as cigale_update.bat not cigale.bat so it doesn't conflict with cigale.exe
    copy /y "%SRC_DIR%\cigale.bat" "%BIN_DIR%\cigale_update.bat" >nul

    :: stage new cigale_cli
    copy /y "%SRC_DIR%\target\release\cigale_cli.exe" "%BIN_DIR%\cigale_pending.exe" >nul
    if %errorlevel% neq 0 ( echo Failed to stage cigale_cli.exe! & exit /b 1 )

    :: schedule rename in hidden window after this process exits
    powershell -Command "Start-Process powershell -WindowStyle Hidden -ArgumentList '-Command sleep 1; if (Test-Path ''%BIN_DIR%\cigale.exe'') { Remove-Item ''%BIN_DIR%\cigale.exe'' -Force }; if (Test-Path ''%BIN_DIR%\cigale.bat'') { Remove-Item ''%BIN_DIR%\cigale.bat'' -Force }; Rename-Item ''%BIN_DIR%\cigale_pending.exe'' ''cigale.exe''; Move-Item ''%BIN_DIR%\cigale_update.bat'' ''%BIN_DIR%\cigale.bat'' -Force;'"

    echo [OK] Binaries installed
    echo     Restart your terminal to complete the update.
    exit /b 0

:install
    echo Installing Cigale...
    call :check_deps
    if %errorlevel% neq 0 exit /b 1

    if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
    if not exist "%SRC_DIR%" mkdir "%SRC_DIR%"

    if exist "%SRC_DIR%\.git" (
        echo Source already exists, fetching latest...
        cd /d "%SRC_DIR%"
        git fetch origin
        if %errorlevel% neq 0 ( echo Failed to fetch! & exit /b 1 )
    ) else (
        echo Cloning repository...
        git clone %REPO_URL% "%SRC_DIR%"
        if %errorlevel% neq 0 ( echo Failed to clone! & exit /b 1 )
        cd /d "%SRC_DIR%"
    )

    if not "%2"=="" (
        echo Checking out version %2...
        git checkout %2
        if %errorlevel% neq 0 ( echo Failed to checkout %2! & exit /b 1 )
    ) else (
        echo Checking out master...
        git checkout master
        if %errorlevel% neq 0 ( echo Failed to checkout master! & exit /b 1 )

        git reset --hard origin/master
        if %errorlevel% neq 0 ( echo Failed to reset master! & exit /b 1 )
    )

    :: Check for .noinstall file in the checked-out branch
    if exist "%SRC_DIR%\.noinstall" (
        echo Error: This branch is marked as not installable ^(.noinstall present^).
        echo        Installation aborted.
        exit /b 1
    )

    echo Building Cigale...
    cd /d "%SRC_DIR%"
    cargo clean
    cargo build --release --bin cigale_nostdl --bin cigale_cli
    if %errorlevel% neq 0 ( echo Build failed! & exit /b 1 )
    cargo build --release --features="stdl" --bin cigale_stdl
    if %errorlevel% neq 0 ( echo Build failed! & exit /b 1 )

    call :copy_binaries
    if %errorlevel% neq 0 exit /b 1

    call :add_to_path

    echo.
    echo [OK] Cigale installed to %BIN_DIR%
    echo     Restart your terminal for PATH changes to take effect.
    echo     Then use: cigale run ^<file.cig^>
    exit /b 0

:update
    echo Updating Cigale...
    call :check_deps
    if %errorlevel% neq 0 exit /b 1

    if not exist "%SRC_DIR%\.git" (
        echo Cigale is not installed. Run: cigale.bat install
        exit /b 1
    )

    echo Pulling latest changes...
    cd /d "%SRC_DIR%"
    git pull
    if %errorlevel% neq 0 ( echo Failed to pull! & exit /b 1 )

    echo Building Cigale...
    cd /d "%SRC_DIR%"
    cargo clean
    cargo build --release --bin cigale_nostdl --bin cigale_cli
    if %errorlevel% neq 0 ( echo Build failed! & exit /b 1 )
    cargo build --release --features="stdl" --bin cigale_stdl
    if %errorlevel% neq 0 ( echo Build failed! & exit /b 1 )

    call :copy_binaries
    if %errorlevel% neq 0 exit /b 1

    echo.
    echo [OK] Cigale updated successfully
    echo     Restart your terminal to complete the update.
    exit /b 0

:uninstall
    echo Uninstalling Cigale...
    powershell -Command "Start-Process powershell -WindowStyle Hidden -ArgumentList '-Command Start-Sleep 1; if (Test-Path \"%INSTALL_DIR%\") { Remove-Item -Recurse -Force \"%INSTALL_DIR%\" }'"

    for /f "tokens=2*" %%a in ('reg query "HKCU\Environment" /v PATH 2^>nul') do set "CURRENT_PATH=%%b"
    set "NEW_PATH=!CURRENT_PATH:%BIN_DIR%;=!"
    set "NEW_PATH=!NEW_PATH:;%BIN_DIR%=!"
    setx PATH "!NEW_PATH!"
    echo [OK] Removed from PATH

    echo.
    echo [OK] Cigale uninstalled
    exit /b 0

:help
    echo Cigale Bootstrap Script
    echo usage: cigale.bat ^<install^|update^|uninstall^> [version]
    echo.
    echo   install [version]  -- install cigale (optionally at a specific version)
    echo   update             -- update cigale to latest
    echo   uninstall          -- remove cigale
    exit /b 1