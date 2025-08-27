#!/bin/bash
RUSTFLAGS="-C link-arg=-s" cargo build --release --lib --target=wasm32-unknown-unknown
if [ ! -d "artifacts" ]; then
  sudo mkdir artifacts
fi
sudo wasm-opt -Os --signext-lowering "target/wasm32-unknown-unknown/release/liquidstaking.wasm" -o "artifacts/liquidstaking.wasm"
