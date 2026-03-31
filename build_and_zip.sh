#!/usr/bin/env bash
set -euo pipefail

PROJECT="sas2-save-editor"
OUT_DIR="out"

WIN_TARGET="x86_64-pc-windows-gnu"
LINUX_TARGET="x86_64-unknown-linux-gnu"

WIN_BIN="target/$WIN_TARGET/release/${PROJECT}.exe"
LINUX_BIN="target/$LINUX_TARGET/release/${PROJECT}"

WIN_ZIP="$OUT_DIR/${PROJECT}-windows.zip"
LINUX_ZIP="$OUT_DIR/${PROJECT}-linux.zip"

echo "[*] Building targets in parallel..."

cargo build --target "$WIN_TARGET" --release &
PID_WIN=$!

cargo build --target "$LINUX_TARGET" --release &
PID_LINUX=$!

wait $PID_WIN
wait $PID_LINUX

echo "[*] Preparing output directory..."
mkdir -p "$OUT_DIR"

echo "[*] Stripping binaries (if possible)..."
if command -v strip >/dev/null 2>&1; then
    strip "$LINUX_BIN" || true
fi

if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
    x86_64-w64-mingw32-strip "$WIN_BIN" || true
elif command -v strip >/dev/null 2>&1; then
    # fallback, may or may not work correctly for PE binaries
    strip "$WIN_BIN" || true
fi

echo "[*] Creating Windows archive..."
zip -j -9 "$WIN_ZIP" "$WIN_BIN"

echo "[*] Creating Linux archive..."
zip -j -9 "$LINUX_ZIP" "$LINUX_BIN"

echo "[✓] Done"
echo "  - $WIN_ZIP"
echo "  - $LINUX_ZIP"
