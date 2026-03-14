@echo off
echo Building cigale_stdl.exe (with stdl)...
cargo build --release --bin cigale_stdl --features stdl
if %errorlevel% neq 0 (
    echo Build failed!
    exit /b %errorlevel%
)

echo Building cigale_nostdl.exe (without stdl)...
cargo build --release --bin cigale_nostdl --no-default-features
if %errorlevel% neq 0 (
    echo Build failed!
    exit /b %errorlevel%
)

echo Done!
echo Binaries are in target/release/