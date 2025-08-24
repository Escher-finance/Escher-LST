#!/usr/bin/env bash

cargo wasm
mkdir artifacts 2>/dev/null
wasm-opt -O3 --signext-lowering "target/wasm32-unknown-unknown/release/cw20_solver.wasm" -o "artifacts/cw20_solver.wasm"
