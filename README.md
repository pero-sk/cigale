# Install Guide

## Linux/Mac
```bash
chmod +x cigale.sh && ./cigale.sh install
```

## Windows
```batch
cigale.bat install
```

## Build from source
```bash
cargo build --release --bin cigale_cli
./target/release/cigale_cli install
```

## Update
```bash
cigale update
```

## Usage
```bash
cigale run <file.cig>
```

## Requirements
- [Rust/Cargo](https://rustup.rs)
- [Git](https://git-scm.com)