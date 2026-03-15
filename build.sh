#!/bin/bash

echo "Building cigale_stdl (with stdl)..."
cargo build --release --bin cigale_stdl --features stdl
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Building cigale_nostdl (without stdl)..."
cargo build --release --bin cigale_nostdl --no-default-features
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Done!"
echo "Binaries are in target/release/"