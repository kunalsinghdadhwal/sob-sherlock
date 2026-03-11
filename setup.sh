#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Decompress block fixtures if not already present
for gz in fixtures/*.dat.gz; do
  [ -f "$gz" ] || continue
  dat="${gz%.gz}"
  if [ ! -f "$dat" ]; then
    echo "Decompressing $(basename "$gz")..."
    gunzip -k "$gz"
  fi
done

# Build Rust project
echo "Building sherlock..."
cargo build --release

echo "Setup complete"
