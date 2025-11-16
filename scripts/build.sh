#!/usr/bin/env bash
set -euo pipefail
TARGET=""
usage() {
  echo "Usage: $0 <linux|windows>" >&2
  exit 1
}
if [[ $# -ne 1 ]]; then
  usage
fi
case "$1" in
  linux)
    TARGET="x86_64-unknown-linux-gnu"
    ;;
  windows)
    TARGET="x86_64-pc-windows-gnu"
    REQUIRED_TOOLS=(x86_64-w64-mingw32-gcc x86_64-w64-mingw32-dlltool)
    for tool in "${REQUIRED_TOOLS[@]}"; do
      if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Missing tool '$tool'. Install mingw-w64 toolchain (e.g., 'brew install mingw-w64' on macOS or 'sudo apt install mingw-w64' on Linux)." >&2
        exit 1
      fi
    done
    ;;
  *)
    usage
    ;;
esac
if ! rustup target list | grep -q "${TARGET} (installed)"; then
  echo "Installing Rust target ${TARGET}"
  rustup target add "${TARGET}"
fi
cargo build --release --target "${TARGET}"
echo "Artifacts available under target/${TARGET}/release/"
