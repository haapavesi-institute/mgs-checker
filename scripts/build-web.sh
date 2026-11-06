#!/usr/bin/env sh
# Builds the WebAssembly package the page loads, into web/pkg.
set -eu
cd "$(dirname "$0")/.."
wasm-pack build crates/mgs-wasm --release --target web --out-dir ../../web/pkg --no-typescript --no-pack
echo "Built web/pkg. Serve the page with: python3 -m http.server -d web 8080"
