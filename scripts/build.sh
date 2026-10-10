#!/bin/zsh
set -euo pipefail
GAME_ROOT="${0:A:h:h}"
cd "$GAME_ROOT"
GAME_TARGET="${CARGO_TARGET_DIR:-$GAME_ROOT/target}"
export CARGO_TARGET_DIR="$GAME_TARGET"
cargo build --locked --release
mkdir -p dist "Neon Clash.app/Contents/MacOS"
cp "$GAME_TARGET/release/neon-clash" dist/neon-clash
cp dist/neon-clash "Neon Clash.app/Contents/MacOS/NeonClash"
cp scripts/Info.plist "Neon Clash.app/Contents/Info.plist"
chmod +x Play.command dist/neon-clash "Neon Clash.app/Contents/MacOS/NeonClash"
if command -v codesign >/dev/null; then
    codesign --force --sign - "Neon Clash.app"
fi
