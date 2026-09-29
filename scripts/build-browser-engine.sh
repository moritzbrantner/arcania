#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(wasm-bindgen --version)" != "wasm-bindgen 0.2.128" ]]; then
  echo 'Install wasm-bindgen-cli 0.2.128 with cargo install wasm-bindgen-cli --version 0.2.128 --locked' >&2
  exit 1
fi
cargo build --locked -p rune-lanes-browser --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir frontend/public/engine --out-name rune_lanes_browser target/wasm32-unknown-unknown/release/rune_lanes_browser.wasm
