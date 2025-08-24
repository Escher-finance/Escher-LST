#!/usr/bin/env bash

mkdir artifacts 2>/dev/null
wasm-opt -Os --signext-lowering "target/wasm32-unknown-unknown/release/cw20_solver.wasm" -o "artifacts/cw20_solver.wasm"
