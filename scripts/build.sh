#!/usr/bin/env bash
# One-stop local build: UI bundle + release binary with the UI embedded.
# npm and cargo are both incremental, so this only rebuilds what changed.
set -euo pipefail
cd "$(dirname "$0")/.."

npm install --prefix ui --no-fund --no-audit
npm run build --prefix ui
cargo build --release -p vanifold-core --features embed-ui

echo ""
echo "single-binary build: target/release/vanifold-core"
